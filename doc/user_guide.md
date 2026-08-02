# Qubit Progress User Guide

`qubit-progress` represents one long-running operation and delivers complete events to a `Reporter`. Start with the [README](../README.md) for a file-copying walkthrough; use this guide when choosing metrics, scheduling reports, or integrating worker threads and output sinks.

## Choose features

The default feature set has no optional dependencies. Enable only the
integration needed by the application:

| Feature | Adds |
| --- | --- |
| `serde` | `Serialize` and `Deserialize` for `Event`, `Phase`, `Stage`, and `MetricSnapshot` |
| `json-lines` | `JsonLinesReporter`; also enables `serde` |
| `log` | `LogReporter` through the `log` facade |

## Core model

An operation has stable configuration and changing state:

- `ProgressBuilder` collects the reporter, report interval, metrics, and optional `Stage` before the operation begins.
- `Metric` is stable metadata: a machine-readable ID, a display name, and an optional total. Every event carries this metadata.
- `Progress` owns the changing state for every configured metric. Obtain a cloneable `MetricHandle` with `Progress::metric` and use it to move quantities through the metric lifecycle.
- `Event` is an immutable, complete observation. A reporter does not need earlier events to reconstruct its state. Enabled operations receive a process-local `operation_id`; `sequence` is zero for `Started` and then counts delivery attempts, including failed attempts.

The lifecycle is `Started → Running* → Succeeded | Failed | Cancelled`. `finish`, `finish_unchecked`, `fail`, and `cancel` consume `Progress`, so safe Rust permits at most one terminal event and no later reports. `finish()` requires zero active work and every known total to be satisfied. If that validation fails, it returns the still-reusable `Progress` together with a `CompletionError`; callers can repair the metrics and retry. Use `finish_unchecked()` only when an intentionally incomplete successful outcome is meaningful. Dropping or unwinding before a terminal call can still abandon an operation without a terminal event.

## Start an operation and update its metrics

Use a stable metric ID in code and a human-readable name for output. A total is optional; when known, configure it once. An operation needs at least one metric; metric IDs and names must be nonblank, and IDs must be unique. A `Stage` also needs a nonblank ID and name, and an optional position must be in the one-based range `1..=total`.

The following operation reports a batch in progress and then completes it.

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

`start()` returns `StartError` for invalid configuration, operation-ID exhaustion, or a rejected `Started` event. `report()` emits the current metric snapshots as `Running` and returns `EmissionError`. A terminal delivery failure is wrapped in `TerminalError`, which retains both elapsed time and the failed event's delivery error.

Event delivery is at-most-once. The crate does not automatically retry a
reporter failure; it returns the error to the caller. If an application retries
at a higher level, the reporter may have accepted the event before returning
the error, so the retry can produce a duplicate. Sinks that need idempotency
should deduplicate using the event's `operation_id` and `sequence`.

For an intentionally unchecked successful close, call `finish_unchecked()`. For
the normal checked successful close, call `finish()`:

```rust
files.start(10)?;
files.succeed(10)?;
progress.finish()?;
```

`finish()` rejects any metric with active work, and rejects a metric with a
known total unless `completed == total`. A rejected finish is recoverable:
`FinishError::Incomplete` returns the operation and a `CompletionError`, while
`FinishError::Terminal` means terminal delivery was attempted and is permanent.

## Metric lifecycle and validation

`start(count)` moves an unsigned quantity from not-started to active. `complete`, `succeed`, `fail`, and `cancel` move unsigned quantities from active to their respective completed states. To undo one of these moves, call `rollback(transition, count)` with the matching `MetricTransition`; it returns terminal work to active, or active work to not-started for `MetricTransition::Start`. All counts remain non-negative. When a total is known, `active + completed` cannot exceed it.

The completed count includes unclassified completion, success, failure, and cancellation. The handle serializes and validates each transition before committing it, so every emitted metric snapshot is internally consistent. When an event contains multiple metrics, each metric snapshot is internally consistent, but the collection is not a globally atomic cross-metric view while the operation is running. Terminal events are stable after the operation closes. Do not hide phase or stage information in counters: use `Stage` at startup, `set_stage` to replace it for future events, and `clear_stage` to remove it.

### Observe compound updates carefully

Each `MetricHandle` method is one atomic state transition, not a transaction
across several method calls. For example, a completed chunk whose items have
different outcomes may be recorded as:

```rust
let completed = 10;
let succeeded = 8;
items.start(completed)?;
items.succeed(succeeded)?;
items.complete(completed - succeeded)?;
```

An automatic reporter can observe a valid intermediate snapshot between these
calls, such as all ten items being active or only eight being completed. Do not
interpret a running event as an atomic business transaction. If a consumer
needs an authoritative aggregate, use a terminal event or synchronize the
business update and report boundary outside `MetricHandle`.

## Close every operation

The type system prevents a second terminal event, but it cannot choose the
right outcome when application work returns an error. Send `Succeeded`,
`Failed`, or `Cancelled` before returning:

```rust
use std::fs;
use qubit_progress::{Metric, Progress, Reporter};

fn copy_one(
    reporter: &dyn Reporter,
    source: &str,
    destination: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let progress = Progress::builder(reporter)
        .metric(Metric::new("files", "Files").total(1))
        .start()?;
    let files = progress.metric("files").expect("configured metric must exist");

    let work_result = (|| -> Result<(), Box<dyn std::error::Error>> {
        files.start(1)?;
        fs::copy(source, destination)?;
        files.succeed(1)?;
        Ok(())
    })();

    match work_result {
        Ok(()) => {
            progress.finish()?;
            Ok(())
        }
        Err(work_error) => {
            progress.fail()?;
            Err(work_error)
        }
    }
}
```

Use `cancel()` instead when the caller or user stops the operation. In the
example, a failed terminal delivery takes precedence over the work error
because `progress.fail()?` returns first. Applications that must retain both
should record or combine the work error before attempting terminal delivery.

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
    records.start(1)?;
    // Process one record.
    records.succeed(1)?;
    progress.report_if_due()?;
}

progress.finish()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

An interval of zero means every `report_if_due()` call is due. Relative
deadlines avoid `Instant` overflow: even `Duration::MAX` is accepted, simply
without a future automatic due deadline. Manual reports and terminal events
remain available.
A reporter failure consumes one delivery sequence number and resets the next
deadline; sequence gaps therefore identify failed delivery attempts rather than
missing state transitions.

## Automatically report state changed by worker threads

`spawn_auto_reporter` reports metric state changed by worker threads. It starts
exactly one scoped background thread for the whole `Progress` operation; any
number of worker threads share the same metric handles and notifier. The
returned `AutoReporter` offers a cloneable `ProgressNotifier` and
`AutoReporterStatus`.

```rust
use std::{thread, time::Duration};
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(std::io::stderr());
let mut progress = Progress::builder(&reporter)
    .interval(Duration::ZERO)
    .metric(Metric::new("files", "Files").total(3))
    .start()?;
let files = progress.metric("files").expect("configured metric must exist");

thread::scope(|scope| -> Result<(), qubit_progress::EmissionError> {
    let auto = progress.spawn_auto_reporter(scope);
    let status = auto.status();
    let notifier = auto.notifier();

    let worker = scope.spawn(move || {
        files.start(3).expect("metric update must succeed");
        // Perform the copy here.
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

The current automatic driver uses one scoped `std::thread` per enabled
operation, independent of the number of workers. It is intended for
thread-based workers and does not integrate with
an async runtime; async callers should drive `report_if_due()` from their own
runtime until a dedicated async driver is added.

`AutoReporterStatus::is_failed()` becomes true when the background reporter exits with an
error or panic. Workers can observe a cloned `AutoReporterStatus` to stop expensive work
early, but `stop()` remains authoritative: it joins the thread, returns its
validation or reporter error, and resumes a panic on the caller thread.

## Disabled operations

Enablement belongs to `Reporter::is_enabled()` and is sampled once by `start()`. A disabled operation still validates its fixed configuration and maintains metric state, but emits no events and starts no automatic reporting thread. This makes an unconditional reporting path cheap when a sink is disabled.

`NoopReporter` is useful when the caller needs a reporter that explicitly disables output. A custom reporter can override `is_enabled()` to connect enablement to application configuration.

## Reporters and structured output

Implement `Reporter` to consume `&Event`; implementations must be `Send + Sync`. Built-in reporters are:

- `TextReporter<W>` writes one human-readable line per event to any `Write + Send` target.
- `NoopReporter` disables events.
- `JsonLinesReporter<W>` is available with `json-lines` and writes one complete JSON event per line.
- `LogReporter` is available with `log` and writes the event's `Debug` representation at info level.

A thread-safe closure with the signature
`Fn(&Event) -> Result<(), ReporterError> + Send + Sync` also implements
`Reporter`, which is often the shortest custom integration.

JSON Lines is suitable for log collectors and post-processing because every
line is a complete event:

```json
{"operation_id":42,"sequence":1,"phase":"running","stage":null,"metrics":[{"id":"files","name":"Files","total":3,"completed":1,"active":1,"succeeded":1,"failed":0,"cancelled":0}],"elapsed":"250ms"}
```

Elapsed duration is encoded as an integer plus the largest exact unit among
`h`, `m`, `s`, `ms`, `us`, and `ns`. Deserialization validates the same public
event invariants as runtime construction.

## Errors and shutdown

`start()` returns `StartError`, running reports return `EmissionError`, and
metric-handle transitions return `MetricError` directly. Do not ignore these
errors: invalid state means no event was delivered, while a sink error means
the delivery attempt failed. `finish()` returns `FinishError`; recover
`CompletionError` by repairing the returned operation, or inspect
`TerminalError` when terminal delivery fails.

Terminal methods return `Result<Duration, TerminalError>`. Inspect `TerminalError` when completion must be recorded reliably: it preserves elapsed time even when the final event cannot be delivered. For automatic reporting, `AutoReporter::stop()` returns `EmissionError` and resumes any worker panic on the caller thread.

When the `serde` feature is enabled, deserializing `Stage`, `MetricSnapshot`, or `Event` validates the same metadata and count invariants used by live progress operations. Treat a deserialization error as invalid external progress data rather than a recoverable event state.

## Further reference

See the generated API documentation on [docs.rs](https://docs.rs/qubit-progress) for every type, error variant, reporter feature, and serialization detail.

## TODO

- Add optional stable operation correlation metadata so shared reporters can
  associate concurrent operations with an application-level job or request.
