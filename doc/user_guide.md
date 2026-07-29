# Qubit Progress User Guide

`qubit-progress` represents one long-running operation and delivers complete events to a `Reporter`. Start with the [README](../README.md) for a file-copying walkthrough; use this guide when choosing metrics, scheduling reports, or integrating worker threads and output sinks.

## Core model

An operation has stable configuration and changing state:

- `ProgressBuilder` collects the reporter, report interval, metrics, and optional `Stage` before the operation begins.
- `Metric` is stable metadata: a machine-readable ID, a display name, and an optional total. Every event carries this metadata.
- `Progress` owns the changing state for every configured metric. Obtain a cloneable `MetricHandle` with `Progress::metric` and use it to move quantities through the metric lifecycle.
- `Event` is an immutable, complete observation. A reporter does not need earlier events to reconstruct its state.

The lifecycle is `Started → Running* → Succeeded | Failed | Cancelled`. `finish`, `fail`, and `cancel` consume `Progress`, so safe Rust permits at most one terminal event and no later reports. Dropping or unwinding before a terminal call can still abandon an operation without a terminal event.

## Start an operation and update its metrics

Use a stable metric ID in code and a human-readable name for output. A total is optional; when known, configure it once. The following operation reports a batch in progress and then completes it.

```rust
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(std::io::stderr());
let mut progress = Progress::builder(&reporter)
    .metric(Metric::new("files", "Files").total(10))
    .start()?;
let files = progress.metric("files").expect("configured metric must exist");

files.start(6)?;
files.succeed(3)?;
files.fail(1)?;
files.complete(1)?;
progress.report()?;

files.start(4)?;
files.succeed(4)?;
let elapsed = progress.finish()?;
# let _ = elapsed;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`start()` validates all fixed metadata before it emits `Started`. `report()` emits the current metric snapshots as `Running`. A terminal call returns the elapsed `Duration`; if terminal delivery fails, `TerminalError` retains both that duration and the underlying `ProgressError`.

## Metric lifecycle and validation

`start(count)` moves positive quantities from not-started to active; a negative count reverses that move. `complete`, `succeed`, `fail`, and `cancel` move positive quantities from active to their respective completed states, while a negative count reverses the corresponding move. All counts must remain non-negative. When a total is known, `active + completed` cannot exceed it; `set_total` cannot lower the total below those occupied quantities.

The completed count includes unclassified completion, success, failure, and cancellation. The handle locks and validates each transition before committing it, so every emitted metric snapshot is internally consistent. Do not hide phase or stage information in counters: use `Stage` at startup, `set_stage` to replace it for future events, and `clear_stage` to remove it.

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
let records = progress.metric("records").expect("configured metric must exist");

for _ in 1..=100 {
    // Process one record.
    records.start(1)?;
    records.succeed(1)?;
    progress.report_if_due()?;
}

progress.finish()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

An interval of zero means every `report_if_due()` call is due. A reporter failure consumes one delivery sequence number and resets the next deadline.

## Automatically report state changed by worker threads

`spawn_auto_reporter` reports metric state changed by worker threads. It starts a scoped background thread that owns the mutable progress borrow. The returned `AutoReporter` offers a cloneable `Notifier` and `Status`.

```rust
use std::{thread, time::Duration};
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(std::io::stderr());
let mut progress = Progress::builder(&reporter)
    .interval(Duration::ZERO)
    .metric(Metric::new("files", "Files").total(3))
    .start()?;
let files = progress.metric("files").expect("configured metric must exist");

thread::scope(|scope| -> Result<(), qubit_progress::ProgressError> {
    let auto = progress.spawn_auto_reporter(scope);
    let status = auto.status();
    let notifier = auto.notifier();

    let worker = scope.spawn(move || {
        files.start(3).expect("metric update must succeed");
        files.succeed(3).expect("metric update must succeed");
        notifier.notify();
    });

    worker.join().expect("copy worker panicked");
    auto.stop()?;
    assert!(!status.is_failed());
    Ok(())
})?;

progress.finish()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

For a zero interval, `notify()` coalesces repeated calls into at most one pending report. For a positive interval, the worker emits heartbeats at the minimum interval and `notify()` is a no-op, avoiding synchronization work for worker threads. `notify()` is harmless after `stop()`. Always call `stop()` and handle its result before calling a terminal method. While the `AutoReporter` exists, the exclusive borrow prevents manual reporting, stage changes, and termination.

## Disabled operations

Enablement belongs to `Reporter::is_enabled()` and is sampled once by `start()`. A disabled operation still validates its fixed configuration and maintains metric state, but emits no events and starts no automatic reporting thread. This makes an unconditional reporting path cheap when a sink is disabled.

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
