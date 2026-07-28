# Qubit Progress User Guide

## Model

One `Progress` value represents one operation. `Metric` is fixed operation configuration: a stable ID, display name, and optional total. `Snapshot` is a temporary mutable view used by a single report closure. It can only change dynamic counts: `completed`, `active`, `succeeded`, and `failed`.

```rust
use qubit_progress::{Metric, Progress, Reporter, ReportError, Event};

struct Sink;
impl Reporter for Sink {
    fn report(&self, _event: &Event) -> Result<(), ReportError> { Ok(()) }
}

let sink = Sink;
let mut progress = Progress::builder(&sink)
    .metric(Metric::new("tasks", "Tasks").total(10))
    .start()?;

progress.report(|snapshot| {
    snapshot.metric("tasks", |counts| {
        counts.completed(4).succeeded(3).failed(1).active(2);
    });
})?;

progress.finish(|snapshot| {
    snapshot.metric("tasks", |counts| {
        counts.completed(10).succeeded(9).failed(1);
    });
})?;
# Ok::<(), qubit_progress::TerminalError>(())
```

Validation prevents duplicate or blank metric IDs, contradictory counts, and counts above a known total. Stage metadata is fixed or updated explicitly with `Stage`, never hidden in counters.

## Scheduling

`report` always attempts a `Running` event. `report_if_due` applies the configured interval and does not invoke its closure before the operation is due. An interval of zero means every call is due. Reporter failures consume a delivery sequence number; invalid snapshots do not.

The terminal methods consume the operation and return its elapsed duration. If terminal delivery fails, `TerminalError` retains both the elapsed duration and the underlying `ProgressError`.

## Disabled operations

Enablement belongs to the reporter and is sampled at `start`. For a disabled operation, report closures are not invoked, terminal closures are not invoked, and automatic reporting allocates neither events nor threads. This lets callers use one unconditional reporting path without `*_if_enabled` variants.

## Automatic reports

Use a scoped reporter for state owned outside the progress object:

```rust
use std::time::Duration;
use qubit_progress::{Metric, NoopReporter, Progress};

let reporter = NoopReporter;
let mut progress = Progress::builder(&reporter)
    .interval(Duration::ZERO)
    .metric(Metric::new("tasks", "Tasks").total(1))
    .start()?;

std::thread::scope(|scope| {
    let auto = progress.spawn_auto_reporter(scope, |snapshot| {
        snapshot.metric("tasks", |counts| counts.completed(1).succeeded(1));
    });
    auto.notifier().notify();
    auto.stop()?;
    Ok::<(), qubit_progress::ProgressError>(())
})?;

progress.finish(|snapshot| {
    snapshot.metric("tasks", |counts| counts.completed(1).succeeded(1));
})?;
# Ok::<(), qubit_progress::TerminalError>(())
```

The scoped exclusive borrow makes it impossible to manually report or terminate while the worker can still report. `Notifier::notify` coalesces calls and is a no-op after `stop`. A positive interval emits heartbeat reports; notifications only wake its wait and do not violate the minimum interval.

## Structured output

With `json-lines`, `JsonLinesReporter` serializes one complete `Event` per line. The elapsed field is an integer plus one of `h`, `m`, `s`, `ms`, `us`, or `ns`; serialization chooses the largest exact unit. Deserialization validates the same public event invariants as runtime construction.
