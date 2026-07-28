# Qubit Progress User Guide

`qubit-progress` represents one long-running operation and delivers complete events to a `Reporter`. Start with the [README](../README.md) for a file-copying walkthrough; use this guide when choosing metrics, scheduling reports, or integrating worker threads and output sinks.

## Core model

An operation has stable configuration and changing state:

- `ProgressBuilder` collects the reporter, report interval, metrics, and optional `Stage` before the operation begins.
- `Metric` is stable metadata: a machine-readable ID, a display name, and an optional total. Every event carries this metadata.
- `Snapshot` exists only inside one report closure. It supplies current `completed`, `active`, `succeeded`, and `failed` counts for each metric.
- `Event` is an immutable, complete observation. A reporter does not need earlier events to reconstruct its state.

The lifecycle is `Started → Running* → Succeeded | Failed | Cancelled`. `finish`, `fail`, and `cancel` consume `Progress`, so safe Rust permits at most one terminal event and no later reports. Dropping or unwinding before a terminal call can still abandon an operation without a terminal event.

## Start an operation and report a snapshot

Use a stable metric ID in code and a human-readable name for output. A total is optional; when known, configure it once. The following operation reports a batch in progress and then completes it.

```rust
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(std::io::stderr());
let mut progress = Progress::builder(&reporter)
    .metric(Metric::new("files", "Files").total(10))
    .start()?;

progress.report(|snapshot| {
    snapshot.metric("files", |counts| {
        counts.completed(4).succeeded(3).failed(1).active(2);
    });
})?;

let elapsed = progress.finish(|snapshot| {
    snapshot.metric("files", |counts| {
        counts.completed(10).succeeded(9).failed(1);
    });
})?;
# let _ = elapsed;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`start()` validates all fixed metadata before it emits `Started`. `report()` validates the new snapshot before it emits `Running`. A terminal call returns the elapsed `Duration`; if terminal delivery fails, `TerminalError` retains both that duration and the underlying `ProgressError`.

## Snapshot rules and validation

Each snapshot must configure every declared metric exactly once. Empty or duplicate metric IDs, an unknown ID in `Snapshot::metric`, a duplicate update, contradictory counts, and values above a configured total are validation errors. In particular, `succeeded + failed` must not exceed `completed`, and `completed + active` must not exceed a known total.

Configure fixed metadata with `Metric` and dynamic values in report closures. Do not hide phase or stage information in counters: use `Stage` at startup, `set_stage` to replace it for future events, and `clear_stage` to remove it. Use `set_total` only when the total becomes known or changes after the operation starts.

## Choose a report schedule

`report()` attempts a `Running` event immediately. `report_if_due()` respects the interval set on the builder:

```rust
use std::time::Duration;
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(std::io::stderr());
let mut progress = Progress::builder(&reporter)
    .interval(Duration::from_secs(1))
    .metric(Metric::new("records", "Records"))
    .start()?;

for completed in 1..=100 {
    // Process one record.
    progress.report_if_due(|snapshot| {
        snapshot.metric("records", |counts| {
            counts.completed(completed).succeeded(completed);
        });
    })?;
}

progress.finish(|snapshot| {
    snapshot.metric("records", |counts| {
        counts.completed(100).succeeded(100);
    });
})?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

An interval of zero means every `report_if_due()` call is due. When an operation is not due, its closure is not invoked. A reporter failure consumes one delivery sequence number and resets the next deadline; a snapshot validation failure does neither.

## Automatically report state changed by worker threads

`spawn_auto_reporter` is for state that lives outside `Progress`, such as counters shared by file-copy workers. It starts a scoped background thread that owns the mutable progress borrow and invokes a snapshot closure. The returned `AutoReporter` offers a cloneable `Notifier` and `Status`.

```rust
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(std::io::stderr());
let mut progress = Progress::builder(&reporter)
    .interval(Duration::ZERO)
    .metric(Metric::new("files", "Files").total(3))
    .start()?;
let completed = Arc::new(Mutex::new(0_u64));

thread::scope(|scope| -> Result<(), qubit_progress::ProgressError> {
    let observed = Arc::clone(&completed);
    let auto = progress.spawn_auto_reporter(scope, move |snapshot| {
        let completed = *observed.lock().expect("progress mutex poisoned");
        snapshot.metric("files", |counts| {
            counts.completed(completed).succeeded(completed);
        });
    });
    let status = auto.status();
    let notifier = auto.notifier();

    let updated = Arc::clone(&completed);
    let worker = scope.spawn(move || {
        *updated.lock().expect("progress mutex poisoned") = 3;
        notifier.notify();
    });

    worker.join().expect("copy worker panicked");
    auto.stop()?;
    assert!(!status.is_failed());
    Ok(())
})?;

progress.finish(|snapshot| {
    snapshot.metric("files", |counts| {
        counts.completed(3).succeeded(3);
    });
})?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

For a zero interval, `notify()` coalesces repeated calls into at most one pending report. For a positive interval, the worker emits heartbeats at the minimum interval and `notify()` is a no-op, avoiding synchronization work for worker threads. `notify()` is harmless after `stop()`. Always call `stop()` and handle its result before calling a terminal method. While the `AutoReporter` exists, the exclusive borrow prevents manual reporting, stage changes, total changes, and termination.

## Disabled operations

Enablement belongs to `Reporter::is_enabled()` and is sampled once by `start()`. A disabled operation still validates its fixed configuration, but it emits no `Started` event; report and terminal closures are not executed; no events are allocated; and automatic reporting starts no background thread. This makes an unconditional reporting path cheap when a sink is disabled.

`NoopReporter` is useful when the caller needs a reporter that explicitly disables output. A custom reporter can override `is_enabled()` to connect enablement to application configuration.

## Reporters and structured output

Implement `Reporter` to consume `&Event`; implementations must be `Send + Sync`. Built-in reporters are:

- `TextReporter<W>` writes one human-readable line per event to any `Write + Send` target.
- `NoopReporter` disables events.
- `JsonLinesReporter<W>` is available with `json-lines` and writes one complete JSON event per line.
- `LogReporter` is available with `log` and forwards events to the `log` ecosystem.

JSON Lines is suitable for log collectors and post-processing because every line is a complete event. Its elapsed duration is encoded as an integer plus the largest exact unit among `h`, `m`, `s`, `ms`, `us`, and `ns`; deserialization validates the same public event invariants as runtime construction.

## Errors and shutdown

Most non-terminal operations return `Result<(), ProgressError>`. A `ProgressError` represents either invalid progress data or a failure returned by the reporter. Do not ignore it: an invalid snapshot means no event was delivered, while a sink error means the delivery attempt failed.

Terminal methods return `Result<Duration, TerminalError>`. Inspect `TerminalError` when completion must be recorded reliably: it preserves elapsed time even when the final event cannot be delivered. For automatic reporting, `AutoReporter::stop()` returns background validation or reporter errors and resumes any worker panic on the caller thread.

## Further reference

See the generated API documentation on [docs.rs](https://docs.rs/qubit-progress) for every type, error variant, reporter feature, and serialization detail.
