# Qubit Progress

`qubit-progress` is a lifecycle-safe protocol for reporting one long-running operation. It separates immutable operation configuration from the changing counts sampled at each report.

Declare every metric and its total once when the operation starts. A report closure supplies only dynamic counts. Every delivered event is complete, so a reporter never needs previous events to reconstruct state.

## Installation

```toml
[dependencies]
qubit-progress = "0.6"
```

Enable `json-lines` for structured JSON output and `log` for the log sink.

## Example

```rust
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::stderr();
let mut progress = Progress::builder(&reporter)
    .metric(Metric::new("files", "Files").total(2))
    .start()?;

progress.report(|snapshot| {
    snapshot.metric("files", |counts| {
        counts.completed(1).succeeded(1).active(1);
    });
})?;

progress.finish(|snapshot| {
    snapshot.metric("files", |counts| {
        counts.completed(2).succeeded(2);
    });
})?;
# Ok::<(), qubit_progress::TerminalError>(())
```

`Started`, `Running`, and the terminal event all include the configured total of `2`; the application never repeats it.

## Lifecycle and enablement

`ProgressBuilder::start` validates fixed configuration, samples `Reporter::is_enabled()` once, and emits `Started` only when enabled. When disabled, every report closure and terminal closure is skipped, no event is created, and no background thread is spawned. Configuration validation still runs.

The lifecycle is `Started → Running* → Succeeded | Failed | Cancelled`. `finish`, `fail`, and `cancel` consume `Progress`, which makes duplicate terminal events and reports after termination impossible in safe Rust.

## Reporters

Implement `Reporter` to receive `&Event`. Built-ins are `NoopReporter`, `TextReporter`, `JsonLinesReporter` (feature `json-lines`), and `LogReporter` (feature `log`). JSON Lines writes exactly one complete event per line and serializes elapsed time as a canonical string such as `"250ms"`.

For worker-driven code, `Progress::spawn_auto_reporter` returns a scoped `AutoReporter`. While it exists, it exclusively borrows the operation. Workers use its cloneable `Notifier` to coalesce zero-interval wakeups; a positive interval produces heartbeat reports. Call `stop()` before the terminal event.

See the [user guide](doc/user_guide.md) and the API documentation for details.
