# Qubit Progress

[![Rust CI](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-progress/coverage-badge.json)](https://qubit-ltd.github.io/rs-progress/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-progress.svg?color=blue)](https://crates.io/crates/qubit-progress)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Generic progress reporting abstractions for the Rust ecosystem.

## When to use this crate

Use `qubit-progress` when an operation needs to report progress without tying
the reporting API to one domain:

- installers that move through preparation, copy, verification, and cleanup
  stages;
- batch jobs that need consistent counters and elapsed time;
- command-line tools that want interchangeable console or log reporters;
- libraries that should expose progress snapshots without depending on a
  concrete runtime.

This crate is not a scheduler, task executor, or UI framework. It only defines
the event model and basic reporter implementations.

## Overview

Qubit Progress models progress as immutable events. A progress event itself carries:

- `ProgressPhase`: lifecycle state such as started, running, finished, failed,
  or canceled.
- `ProgressStage`: optional stage metadata for multi-stage operations.
- `ProgressCounters`: generic total, completed, active, succeeded, and failed
  counters.
- elapsed time as `std::time::Duration`.
In addition, this crate provides:

- `ProgressEventBuilder`: fluent builder for constructing progress events
  without manually assembling counters and stage metadata first.
- `ProgressReporter`: trait for receiving progress events.
- `Progress`: helper for reporting a single operation's lifecycle with
  elapsed time and interval-based running updates.
- `RunningProgressGuard` and `RunningProgressPointHandle`: helpers returned by
  `Progress::spawn_running_reporter` for background `running` reports while
  workers update shared domain state.
- `NoOpProgressReporter`, `StdoutProgressReporter`,
  `StderrProgressReporter`, `WriterProgressReporter`, and
  `LoggerProgressReporter`: reusable reporter implementations.

`ProgressReporter` is intentionally small: when the stock reporters are not
enough, your application can provide its own implementation—for example, to
drive a graphical progress bar, refresh a status region in a desktop UI, or
forward updates to a web client. This crate stays UI-agnostic; wiring events to
a particular toolkit or transport is your integration layer.

Domain crates should keep their own domain state and expose progress by
converting their state into `ProgressEvent` values. Domain-specific errors,
logs, metrics, and traces should stay in their own mechanisms instead of being
attached to progress events.

## Installation

```toml
[dependencies]
qubit-progress = "0.4.3"
```

## Quick Start

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressEvent,
    ProgressReporter,
    StdoutProgressReporter,
};

let reporter = StdoutProgressReporter::default();
let event = ProgressEvent::builder()
    .running()
    .total(4)
    .completed(2)
    .active(1)
    .stage_named("copy", "Copy files")
    .elapsed(Duration::from_secs(2))
    .build();

reporter.report(&event);
```

The lower-level constructors remain available when a caller already has
prebuilt counters or stage metadata, but the builder is the preferred entry
point for ordinary reporting code.

## Reporting an operation lifecycle

Use `Progress` when one operation needs a started event, periodic running
events, and a terminal event. The run tracks elapsed time and throttles
`running` callbacks, while your domain code owns the counters.

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressCounters,
    Progress,
    StdoutProgressReporter,
};

let reporter = StdoutProgressReporter::default();
let mut progress = Progress::new(&reporter, Duration::from_secs(5));

let started = progress.report_started(ProgressCounters::new(Some(3)));
assert!(started.elapsed().is_zero());

let mut completed = 0;
for _task in 0..3 {
    // ... execute one unit of work ...
    completed += 1;
    let counters = ProgressCounters::new(Some(3)).with_completed_count(completed);
    let _running_event = progress.report_running_if_due(counters);
}

let final_counters = ProgressCounters::new(Some(3))
    .with_completed_count(3)
    .with_succeeded_count(3);
let finished = progress.report_finished(final_counters);
assert!(finished.elapsed() >= started.elapsed());
```

`report_running_if_due` returns `Some(event)` only when it emitted an event. The
method does not block waiting for the next interval: it returns `None`
immediately when not due, and when due it synchronously calls the reporter and
returns the emitted event (so blocking behavior depends on the reporter implementation).
A practical pattern is calling it once after each completed unit of work:
reporting happens automatically when the interval is due, and otherwise the
call is effectively a no-op.
Use `report_running` when an external scheduler or background thread already
controls the reporting interval.

## Reporting from a background thread

Use `Progress::spawn_running_reporter` when work happens on one or more worker
threads but `running` events should be reported from a separate background
thread. Workers keep updating domain state. The background reporter waits for
either a timeout or a `RunningProgressPointHandle::report` signal, then calls
your snapshot closure to build fresh `ProgressCounters`.

This is useful for parallel executors: reporter callbacks stay out of worker
hot paths for positive intervals, while `Duration::ZERO` can still report
after each worker progress point without busy waiting. Keep the
`RunningProgressGuard` on the coordinating thread, pass
`RunningProgressPointHandle` clones to workers, and call `stop_and_join` before
terminal `finished`, `failed`, or `canceled` events are reported.

The example below uses [`qubit-atomic`](https://crates.io/crates/qubit-atomic)’s
[`AtomicCount`](https://docs.rs/qubit-atomic/latest/qubit_atomic/struct.AtomicCount.html)
for the shared completion counter. Add it to your manifest only when you adopt
this pattern:

```toml
qubit-atomic = "0.10"
```

```rust
use std::{
    sync::Arc,
    thread,
    time::Duration,
};

use qubit_atomic::AtomicCount;
use qubit_progress::{
    Progress,
    ProgressCounters,
    StdoutProgressReporter,
};

let reporter = StdoutProgressReporter::default();
let completed = Arc::new(AtomicCount::zero());

thread::scope(|scope| {
    let loop_completed = Arc::clone(&completed);
    let progress = Progress::new(&reporter, Duration::ZERO);
    let running_progress =
        progress.spawn_running_reporter(scope, move || {
            ProgressCounters::new(Some(3))
                .with_completed_count(loop_completed.get())
        });
    let progress_point_handle = running_progress.point_handle();

    let mut handles = Vec::new();
    for _ in 0..3 {
        let completed_worker = Arc::clone(&completed);
        let point = progress_point_handle.clone();
        handles.push(scope.spawn(move || {
            completed_worker.inc();
            point.report();
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }

    running_progress.stop_and_join();
});
```

For positive intervals, `RunningProgressPointHandle::report` is a no-op; the
background reporter wakes itself with a timeout. This lets worker code call it
unconditionally while the guard keeps stop/join ownership on the coordinating
thread.

## Multi-stage progress

Stages describe where an operation is inside a larger workflow. They are
separate from lifecycle phases: a copy stage can be running, finished, failed,
or canceled depending on the event.

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressEvent,
    ProgressPhase,
    ProgressStage,
};

let stage = ProgressStage::new("verify", "Verify installation")
    .with_index(2)
    .with_total_stages(5)
    .with_weight(0.2);

let event = ProgressEvent::builder()
    .phase(ProgressPhase::Running)
    .total(10)
    .completed(7)
    .elapsed(Duration::from_secs(12))
    .stage(stage)
    .build();

assert_eq!(event.phase(), ProgressPhase::Running);
assert_eq!(event.counters().completed_count(), 7);
```

## Counter semantics

`ProgressCounters` supports known-total and unknown-total progress.

- `total_count: Some(n)` means percentage and remaining count can be computed.
- `total_count: None` means the operation is open-ended or the total is not yet
  known.
- `completed_count` is the amount of work that reached a terminal state.
- `active_count` is the amount of work currently in flight.
- `succeeded_count` and `failed_count` are optional aggregate outcome counters
  for domains that can report them.

For zero-sized known-total work, progress is treated as complete:

```rust
use qubit_progress::ProgressCounters;

let counters = ProgressCounters::new(Some(0));
assert_eq!(counters.progress_percent(), Some(100.0));
```

## Reporter behavior

Reporter callbacks are intentionally side-effecting. A reporter may write to a
terminal, append to a file, emit logs, update a UI bridge, or record events for
tests. If a reporter panics, the caller decides whether to propagate or isolate
that panic.

`WriterProgressReporter` writes a compact human-readable line.

`StdoutProgressReporter` and `StderrProgressReporter` are convenience
reporters built on top of `WriterProgressReporter` for standard output and
standard error.

`LoggerProgressReporter` emits through the `log` crate and can be configured
with a target and level.

## Public API Cheat Sheet

- `ProgressPhase`: lifecycle phase enum.
- `ProgressStage`: stage id, name, index, total stage count, and optional
  weight.
- `ProgressCounters`: generic counters with remaining-count and percentage
  helpers.
- `ProgressEvent`: immutable event carrying phase, stage, counters, and elapsed
  time.
- `ProgressEventBuilder`: fluent builder for event construction.
- `ProgressReporter`: trait for receiving progress events.
- `Progress`: lifecycle helper for one progress-producing operation.
- `RunningProgressGuard`: guard that owns a scoped background reporter thread.
- `RunningProgressPointHandle`: cloneable worker-side handle for running
  points.
- `NoOpProgressReporter`: reporter that ignores events.
- `StdoutProgressReporter`: stdout convenience reporter.
- `StderrProgressReporter`: stderr convenience reporter.
- `WriterProgressReporter<W>`: writer-backed human-readable reporter.
- `LoggerProgressReporter`: `log` crate-backed reporter.

## Documentation

- API documentation: [docs.rs/qubit-progress](https://docs.rs/qubit-progress)
- Crate package: [crates.io/crates/qubit-progress](https://crates.io/crates/qubit-progress)
- Source repository: [github.com/qubit-ltd/rs-progress](https://github.com/qubit-ltd/rs-progress)

## Testing and CI

Run the fast local checks from the crate root:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

To match the repository CI environment, run:

```bash
./align-ci.sh
./ci-check.sh
./coverage.sh json
```

`./align-ci.sh` aligns the local toolchain and CI-related configuration before
`./ci-check.sh` runs the same checks used by the pipeline. Use `./coverage.sh`
when changing behavior that should be reflected in coverage reports.

## Contributing

Issues and pull requests are welcome. Please keep changes focused, add or update
tests when behavior changes, and update this README or rustdoc when public API
or user-visible behavior changes.

By contributing, you agree that your contribution is licensed under the same
[Apache License, Version 2.0](LICENSE) as this project.

## License and Copyright

Copyright © 2026 Haixing Hu, Qubit Co. Ltd.

This software is licensed under the [Apache License, Version 2.0](LICENSE).

## Author and Maintenance

**Haixing Hu** — Qubit Co. Ltd.

| | |
| --- | --- |
| **Repository** | [github.com/qubit-ltd/rs-progress](https://github.com/qubit-ltd/rs-progress) |
| **API documentation** | [docs.rs/qubit-progress](https://docs.rs/qubit-progress) |
| **Crate** | [crates.io/crates/qubit-progress](https://crates.io/crates/qubit-progress) |
