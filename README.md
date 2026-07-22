# Qubit Progress

[![Rust CI](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-progress/coverage-badge.json)](https://qubit-ltd.github.io/rs-progress/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-progress.svg?color=blue)](https://crates.io/crates/qubit-progress)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Generic progress reporting abstractions for Qubit Rust libraries and applications.

## Overview

Qubit Progress models progress as immutable, self-describing events. Each event
carries a metric schema, lifecycle phase, optional stage information, metric
counters, and elapsed time. Reporters receive complete event snapshots and can
render them as human-readable text, logs, JSON, or application-specific records.

Use this crate when you need:

- stable metric definitions for an operation, such as files, bytes, entries, or tasks;
- `u64` counters grouped by metric id;
- lifecycle phases such as `started`, `running`, `finished`, `failed`, and `canceled`;
- optional stage metadata for multi-step operations;
- operation-scoped reporting with configurable running-report intervals;
- background running reporters for worker-driven operations;
- serde-serializable progress events suitable for logs, agents, and structured consumers.

For detailed usage, extension examples, and reporter design guidance, see the [User Guide](doc/user_guide.md).
API reference documentation is available on [docs.rs](https://docs.rs/qubit-progress).

## Installation

```toml
[dependencies]
qubit-progress = "0.6"
```

## Quick Example

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressEvent,
    ProgressMetric,
    ProgressPhase,
    ProgressSchema,
};

let schema = ProgressSchema::new(vec![
    ProgressMetric::new("entries", "Entries"),
    ProgressMetric::new("bytes", "Bytes"),
]);

let event = ProgressEvent::builder(schema)
    .running()
    .stage_named("copy", "Copy files")
    .counter("entries", |counter| counter.total(10).completed(4))
    .counter("bytes", |counter| counter.total(1_000_000).completed(400_000))
    .elapsed(Duration::from_millis(110))
    .build();

assert_eq!(event.phase(), ProgressPhase::Running);
assert_eq!(event.counter("entries").map(|c| c.completed_count()), Some(4));
```

## Main Capabilities

### Schema and Metrics

`ProgressSchema` defines the metric dimensions that may appear in a progress
event stream. A metric has a stable `id` for structured data and a human-readable
`name` for display output.

| Type | Purpose |
| --- | --- |
| `ProgressSchema` | metric definitions for one logical operation |
| `ProgressMetric` | stable metric id plus display name |
| `ProgressCounter` | `u64` counts for one metric id |
| `ProgressMetricSnapshot` | one metric counter flattened with event phase, stage, and elapsed time |
| `ProgressStage` | optional multi-stage operation metadata |

A schema can contain multiple metrics, for example `entries` and `bytes`, so a
single event can report logical item progress and byte progress together without
mixing their units.

### Events and Counters

A `ProgressEvent` is an immutable snapshot. It contains:

| Field | Purpose |
| --- | --- |
| `schema` | metric definitions carried with the event |
| `phase` | lifecycle state: `started`, `running`, `finished`, `failed`, or `canceled` |
| `stage` | optional multi-stage operation metadata |
| `counters` | one or more `ProgressCounter` values grouped by `metric_id` |
| `elapsed` | elapsed `Duration`, serialized by `qubit-serde` as strings such as `110ms` |

Use `ProgressEvent::builder(schema)` to build events directly:

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressEvent,
    ProgressMetric,
    ProgressSchema,
};

let event = ProgressEvent::builder(ProgressSchema::single("entries", "Entries"))
    .running()
    .counter("entries", |counter| counter.total(5).completed(2))
    .elapsed(Duration::from_millis(110))
    .build();

assert_eq!(event.counter("entries").map(|counter| counter.completed_count()), Some(2));
```

### Operation-Scoped Progress

A `Progress` instance is scoped to one logical operation. Do not mix unrelated
operations into one reporter stream unless the reporter explicitly implements
multiplexing. Multi-threaded work should aggregate counters and report them
through the operation-scoped `Progress`.

```rust
use std::time::Duration;

use qubit_progress::{
    Progress,
    ProgressMetric,
    ProgressSchema,
    WriterProgressReporter,
};

let schema = ProgressSchema::new(vec![
    ProgressMetric::new("entries", "Entries"),
    ProgressMetric::new("bytes", "Bytes"),
]);
let reporter = WriterProgressReporter::from_writer(std::io::stdout());
let mut progress = Progress::new(&reporter, Duration::from_secs(1), schema);

progress
    .report_started(|event| event.counter("entries", |counter| counter.total(3)))
    .expect("progress output should succeed");

progress.report_running(|event| {
    event
        .counter("entries", |counter| counter.total(3).completed(1).active(1))
        .counter("bytes", |counter| counter.total(1_024).completed(512))
}).expect("progress output should succeed");

progress.report_finished(|event| {
    event
        .counter("entries", |counter| counter.total(3).completed(3).succeeded(3))
        .counter("bytes", |counter| counter.total(1_024).completed(1_024))
}).expect("progress output should succeed");
```

`report_running_if_due` only invokes the builder closure when the configured
interval has elapsed. This keeps hot paths cheap for positive intervals.

### Background Reporter Thread

Use `Progress::spawn_running_reporter` when worker threads update domain state
and a coordinating thread should emit periodic `running` events. Workers update
shared state, then call `RunningProgressPointHandle::report()` to wake the
background reporter thread when the interval is `Duration::ZERO`.
`RunningProgressGuard` owns that background reporter thread, and
`RunningProgressPointHandle` is the cloneable worker-side wakeup handle.

The example below uses `qubit-atomic`'s `ArcAtomic`; add
`qubit-atomic = "0.13"` if you copy this pattern into your own crate.

```rust
use std::{
    thread,
    time::Duration,
};

use qubit_atomic::ArcAtomic;
use qubit_progress::{
    NoOpProgressReporter,
    Progress,
    ProgressCounter,
    ProgressSchema,
};

let reporter = NoOpProgressReporter;
let completed = ArcAtomic::new(0u64);
let progress = Progress::new(
    &reporter,
    Duration::ZERO,
    ProgressSchema::single("entries", "Entries"),
);

thread::scope(|scope| {
    let snapshot_completed = completed.clone();
    let running = progress.spawn_running_reporter(scope, move || {
        vec![ProgressCounter::new("entries")
            .total(3)
            .completed(snapshot_completed.load())]
    });
    let point = running.point_handle();

    completed.store(1);
    point.report();

    running.stop_and_join().expect("progress output should succeed");
});
```

### Reporter Implementations

Reporters receive immutable `ProgressEvent` values through `ProgressReporter`:

```rust
fn report(&self, event: &ProgressEvent) -> Result<(), ProgressReportError>;
```

Built-in reporters:

| Reporter | Purpose |
| --- | --- |
| `NoOpProgressReporter` | ignores events |
| `MetricSnapshotProgressReporter` | sends structured `ProgressMetricSnapshot` objects to a consumer |
| `FormattedProgressReporter` | formats each metric snapshot and sends strings to a consumer |
| `HumanReadableProgressReporter` | sends human-readable metric snapshot strings to a consumer |
| `JsonProgressReporter` | sends JSON metric snapshot strings to a consumer |
| `WriterProgressReporter` | writes human-readable metric snapshot lines to any `Write` sink |
| `StdoutProgressReporter` | writes to stdout |
| `StderrProgressReporter` | writes to stderr |
| `LoggerProgressReporter` | emits through the `log` crate |
| `JsonWriterProgressReporter` | writes JSON metric snapshot lines to any `Write` sink |
| `JsonStdoutProgressReporter` | writes JSON metric snapshots to stdout |
| `JsonStderrProgressReporter` | writes JSON metric snapshots to stderr |
| `JsonLoggerProgressReporter` | emits JSON metric snapshots through the `log` crate |

A reporter can call `event.metric_snapshots_iter()` to lazily turn each counter into a
`ProgressMetricSnapshot` containing the complete metric object, phase, optional
stage, flattened counter values, and elapsed time.

## JSON Serialization

Progress events are serde-serializable. `elapsed` uses the `duration_with_unit`
adapter from `qubit-serde`, so JSON is compact and agent-friendly.
The adapter selects the largest unit that preserves the value exactly, so
sub-millisecond elapsed times round-trip without precision loss. Deserialization
requires canonical duration text and does not trim surrounding whitespace.

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressEvent,
    ProgressMetric,
    ProgressSchema,
};

let schema = ProgressSchema::new(vec![ProgressMetric::new("entries", "Entries")]);
let event = ProgressEvent::builder(schema)
    .running()
    .counter("entries", |counter| counter.total(5).completed(2))
    .elapsed(Duration::from_millis(110))
    .build();

let json = serde_json::to_string(&event).expect("event should serialize");
assert_eq!(
    json,
    concat!(
        r#"{"schema":{"metrics":["#,
        r#"{"id":"entries","name":"Entries"}"#,
        r#"]},"phase":"running","counters":["#,
        r#"{"metric_id":"entries","total_count":5,"completed_count":2,"#,
        r#""active_count":0,"succeeded_count":0,"failed_count":0}"#,
        r#"],"elapsed":"110ms"}"#,
    ),
);
```

## Crate Boundary

`qubit-progress` provides progress data models, operation-scoped lifecycle
helpers, and reporter abstractions. It intentionally does not provide terminal
UI widgets, async runtime integration, task scheduling, tracing infrastructure,
or long-term metrics storage.

## Runtime Dependencies

The core crate depends on:

- `serde` for serializable progress models;
- `qubit-serde` for compact `Duration` serialization.

Default features additionally enable `consumer-reporters`, `json`, and `log`.
Disable default features for the core model, lifecycle APIs, no-op reporter, and
writer reporters without `qubit-function`, `serde_json`, or `log`:

```toml
qubit-progress = { version = "0.6", default-features = false }
```

The crate does not require an async runtime.

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./align-ci.sh` to format code and
`./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-progress](https://github.com/qubit-ltd/rs-progress)
