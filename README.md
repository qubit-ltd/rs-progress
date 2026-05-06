# Qubit Progress

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-progress.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-progress)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-progress/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-progress?branch=main)
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

Qubit Progress models progress as immutable events. A progress event carries:

- `ProgressPhase`: lifecycle state such as started, running, finished, failed,
  or canceled.
- `ProgressStage`: optional stage metadata for multi-stage operations.
- `ProgressCounters`: generic total, completed, active, succeeded, and failed
  counters.
- elapsed time as `std::time::Duration`.
- `ProgressEventBuilder`: fluent builder for constructing progress events
  without manually assembling counters and stage metadata first.
- `ProgressReporter`: trait for receiving progress events.
- `NoOpProgressReporter`, `WriterProgressReporter`, and
  `LoggerProgressReporter`: reusable reporter implementations.

Domain crates should keep their own domain state and expose progress by
converting their state into `ProgressEvent` values. Domain-specific errors,
logs, metrics, and traces should stay in their own mechanisms instead of being
attached to progress events.

## Installation

```toml
[dependencies]
qubit-progress = "0.2"
```

## Quick Start

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressEvent,
    ProgressReporter,
    WriterProgressReporter,
};

let reporter = WriterProgressReporter::from_writer(Vec::<u8>::new());
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
- `NoOpProgressReporter`: reporter that ignores events.
- `WriterProgressReporter<W>`: writer-backed human-readable reporter.
- `LoggerProgressReporter`: `log` crate-backed reporter.

## Project Layout

- `src/progress_phase.rs`: lifecycle phase enum.
- `src/progress_stage.rs`: multi-stage metadata.
- `src/progress_counters.rs`: generic counter model.
- `src/progress_event.rs`: immutable event type.
- `src/progress_event_builder.rs`: fluent event builder.
- `src/progress_reporter.rs`: reporter trait.
- `src/writer_progress_reporter.rs`: writer-backed reporter.
- `src/logger_progress_reporter.rs`: logger-backed reporter.
- `tests/progress`: behavior tests for counters, events, and reporters.

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
