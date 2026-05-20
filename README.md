# Qubit Progress

`qubit-progress` provides lightweight progress reporting primitives for Qubit
Rust libraries and applications.

It models progress as immutable, self-describing events. Each event contains:

- `ProgressSchema`: metric definitions for the logical operation.
- `ProgressMetric`: a stable metric `id` and human-readable `name`.
- `ProgressCounter`: `u64` counters for one metric.
- `ProgressPhase`: `started`, `running`, `finished`, `failed`, or `canceled`.
- `ProgressStage`: optional multi-stage operation metadata.
- `elapsed`: a `std::time::Duration` serialized by `qubit-serde` as strings such
  as `110ms`.

A `Progress` instance is scoped to one logical operation. Do not mix unrelated
operations into one reporter stream unless the reporter explicitly implements
multiplexing. Multi-threaded work should aggregate counters and report them
through the operation-scoped `Progress`.

## Installation

```toml
[dependencies]
qubit-progress = "0.5"
```

## Build a self-describing event

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

## Report one operation

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

progress.report_started(|event| event.counter("entries", |counter| counter.total(3)));

progress.report_running(|event| {
    event
        .counter("entries", |counter| counter.total(3).completed(1).active(1))
        .counter("bytes", |counter| counter.total(1_024).completed(512))
});

progress.report_finished(|event| {
    event
        .counter("entries", |counter| counter.total(3).completed(3).succeeded(3))
        .counter("bytes", |counter| counter.total(1_024).completed(1_024))
});
```

`report_running_if_due` only invokes the builder closure when the configured
interval has elapsed. This keeps hot paths cheap for positive intervals.

## Background reporter thread

Use `Progress::spawn_running_reporter` when worker threads update domain state
and a coordinating thread should emit periodic `running` events. Workers update
shared state, then call `RunningProgressPointHandle::report()` to wake the
background reporter thread when the interval is `Duration::ZERO`.
`RunningProgressGuard` owns that background reporter thread, and
`RunningProgressPointHandle` is the cloneable worker-side wakeup handle.

```rust
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use qubit_progress::{
    NoOpProgressReporter,
    Progress,
    ProgressCounter,
    ProgressSchema,
};

let reporter = NoOpProgressReporter;
let completed = Arc::new(AtomicU64::new(0));
let progress = Progress::new(
    &reporter,
    Duration::ZERO,
    ProgressSchema::single("entries", "Entries"),
);

thread::scope(|scope| {
    let snapshot_completed = Arc::clone(&completed);
    let running = progress.spawn_running_reporter(scope, move || {
        vec![ProgressCounter::new("entries")
            .total(3)
            .completed(snapshot_completed.load(Ordering::Acquire))]
    });
    let point = running.point_handle();

    completed.store(1, Ordering::Release);
    assert!(point.report());

    running.stop_and_join();
});
```

## JSON serialization

Progress events are serde-serializable. `elapsed` uses the `duration_with_unit`
adapter from `qubit-serde`, so JSON is compact and agent-friendly.

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
assert!(json.contains("\"elapsed\":\"110ms\""));
```

## Reporter implementations

- `NoOpProgressReporter`: ignores events.
- `WriterProgressReporter`: writes human-readable lines to any `Write` sink.
- `StdoutProgressReporter`: writes to stdout.
- `StderrProgressReporter`: writes to stderr.
- `LoggerProgressReporter`: emits through the `log` crate.

Reporters receive `ProgressEvent` through:

```rust
fn report(&self, event: &ProgressEvent);
```

A reporter can group counters by `metric_id` and use `event.schema()` to resolve
human-readable names.
