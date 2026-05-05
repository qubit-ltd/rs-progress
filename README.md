# Qubit Progress

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-progress.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-progress)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-progress/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-progress?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-progress.svg?color=blue)](https://crates.io/crates/qubit-progress)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Generic progress reporting abstractions for the Qubit Rust libraries.

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
- caller-defined context for domain-specific details.
- `ProgressReporter<C>`: trait for receiving typed progress events.
- `NoOpProgressReporter`, `WriterProgressReporter`, and
  `LoggerProgressReporter`: reusable reporter implementations.

Domain crates should keep their own domain state and expose progress by
converting it into `ProgressEvent<C>` snapshots. For example, `qubit-batch` can
attach a batch progress snapshot as the event context while still reusing the
generic progress lifecycle and reporter plumbing.

## Installation

```toml
[dependencies]
qubit-progress = "0.1"
```

## Quick Start

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressCounters,
    ProgressEvent,
    ProgressReporter,
    ProgressStage,
    WriterProgressReporter,
};

let reporter = WriterProgressReporter::from_writer(Vec::<u8>::new());
let counters = ProgressCounters::new(Some(4))
    .with_completed_count(2)
    .with_active_count(1);
let event = ProgressEvent::running(counters, Duration::from_secs(2), ())
    .with_stage(ProgressStage::new("copy", "Copy files"));

reporter.report(&event);
```

## Multi-stage progress

Stages describe where an operation is inside a larger workflow. They are
separate from lifecycle phases: a copy stage can be running, finished, failed,
or canceled depending on the event.

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressCounters,
    ProgressEvent,
    ProgressPhase,
    ProgressStage,
};

let stage = ProgressStage::new("verify", "Verify installation")
    .with_index(2)
    .with_total_stages(5)
    .with_weight(0.2);

let event = ProgressEvent::new(
    ProgressPhase::Running,
    ProgressCounters::new(Some(10)).with_completed_count(7),
    Duration::from_secs(12),
    "checking files",
)
.with_stage(stage);

assert_eq!(event.phase(), ProgressPhase::Running);
assert_eq!(event.context(), &"checking files");
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

`WriterProgressReporter` writes a compact human-readable line. It ignores the
event context, so it can report any `ProgressEvent<C>`.

`LoggerProgressReporter` emits through the `log` crate and can be configured
with a target and level.

## Public API Cheat Sheet

- `ProgressPhase`: lifecycle phase enum.
- `ProgressStage`: stage id, name, index, total stage count, and optional
  weight.
- `ProgressCounters`: generic counters with remaining-count and percentage
  helpers.
- `ProgressEvent<C>`: immutable event carrying phase, stage, counters, elapsed
  time, and caller context.
- `ProgressReporter<C>`: trait for receiving progress events.
- `NoOpProgressReporter`: reporter that ignores events.
- `WriterProgressReporter<W>`: writer-backed human-readable reporter.
- `LoggerProgressReporter`: `log` crate-backed reporter.

## Project Layout

- `src/progress_phase.rs`: lifecycle phase enum.
- `src/progress_stage.rs`: multi-stage metadata.
- `src/progress_counters.rs`: generic counter model.
- `src/progress_event.rs`: immutable event type.
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
