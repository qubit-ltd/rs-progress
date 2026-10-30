# Qubit Progress User Guide

`qubit-progress` represents one long-running operation and delivers complete events to a `Reporter`. Start with the [README](../README.md) for a file-copying walkthrough; use this guide when choosing metrics, scheduling reports, or integrating worker threads and output sinks.

[中文](user_guide.zh_CN.md)

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

- `ProgressBuilder` collects the reporter, report interval, metrics, optional `Stage`, and immutable operation attributes before the operation begins.
- `Metric` is stable metadata: a machine-readable ID, a display name, and an optional total. Every event carries this metadata.
- `Progress` owns the changing state for every configured metric. Obtain a cloneable `MetricHandle` with `Progress::metric` and use it to move quantities through the metric lifecycle.
- `Event` is an immutable, complete observation. A reporter does not need earlier events to reconstruct its state. Enabled operations receive a process-local `operation_id`; `sequence` is zero for `Started` and then counts delivery attempts, including failed attempts. `OperationAttributes` carries optional stable string key-value correlation data through every event.

The lifecycle is `Started → Running* → Succeeded | Failed | Cancelled`. `finish`, `finish_recoverable`, `finish_unchecked`, `fail`, and `cancel` consume `Progress`, so safe Rust permits at most one terminal event and no later reports. `finish()` requires zero active work and every known total to be satisfied. Its `FinishError` is static and can be propagated with `?`; when completion validation fails, the operation is consumed. Use `finish_recoverable()` only when the caller must repair metrics and retry. Use `finish_unchecked()` only when an intentionally incomplete successful outcome is meaningful. Dropping or unwinding before a terminal call can still abandon an operation without a terminal event.

## Start an operation and update its metrics

Use a stable metric ID in code and a human-readable name for output. A total is optional; when known, configure it once. An operation needs at least one metric; metric IDs and names must be nonblank, and IDs must be unique. A `Stage` also needs a nonblank ID and name, and an optional position must be in the one-based range `1..=total`.

The following operation reports a batch in progress and then completes it.

```rust
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(std::io::stderr());
let mut progress = Progress::builder(&reporter)
    .attribute("job_id", "import-42")
    .metric(Metric::new("files", "Files").total(10))
    .start()?;
let files = progress.metric("files").expect("configured metric must exist");

files.start(6)?;
files.succeed(3)?;
files.fail(1)?;
files.complete(2)?;
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
should deduplicate using `(process_instance_id, operation_id, sequence)`. The
application generates `process_instance_id` and keeps it stable while replaying
an event, while assigning a new value to each concurrent progress instance
and after a process restart. It can be carried in `OperationAttributes` or an
external envelope. A task ID and attempt are suitable only when they uniquely
identify the process and progress instance; do not assume that they do.

Replaying the original `Event` from `DeliveryError::event()` or
`DeliveryError::into_parts()` preserves its key. Calling `report()` creates a
new sequence, and starting again creates a new operation ID; after restart,
that ID may also repeat. These calls do not identify a retry as a replay of the
same business event. The sink owns its idempotency policy.

For an intentionally unchecked successful close, call `finish_unchecked()`. For
the normal checked successful close, call `finish()`:

```rust
files.start(10)?;
files.succeed(10)?;
progress.finish()?;
```

`finish()` rejects any metric with active work, and rejects a metric with a
known total unless `completed == total`. `FinishError::Incomplete` contains the
completion error and the elapsed time sampled before validation after the
operation has been consumed. `FinishError::elapsed()` returns that same value
for either incomplete validation or terminal delivery failure. This value is
the elapsed operation time sampled at the beginning of the terminal attempt,
before freezing updates, validating completion, or delivering the terminal
event. It is not the duration of the `finish()` call and excludes subsequent
waiting for terminal delivery. Use
`finish_recoverable()` when a caller needs repair-and-retry behavior; its
`RecoverableFinishError::Incomplete` contains the reusable operation.
`FinishError::Terminal` means terminal delivery was attempted and is permanent.
The guide's first example completes ten files across successful, failed, and
unclassified work; `finish()` checks the total completed count, not that every
file succeeded.

## Metric lifecycle and validation

`start(count)` moves an unsigned quantity from not-started to active. `complete`, `succeed`, `fail`, and `cancel` move unsigned quantities from active to their respective completed states. Completed work is the sum of unclassified, succeeded, failed, and cancelled work. Metrics are monotonic; `rollback()` is not available. All counts remain non-negative. When a total is known, `active + completed` cannot exceed it.

The completed count includes unclassified completion, success, failure, and cancellation. The handle serializes and validates each transition before committing it, so every emitted metric snapshot is internally consistent. When an event contains multiple metrics, each metric snapshot is internally consistent, but the collection is not a globally atomic cross-metric view while the operation is running. Terminal events are stable after the operation closes. Do not hide phase or stage information in counters: use `Stage` at startup, `set_stage` to replace it for future events, and `clear_stage` to remove it.

### Apply compound updates atomically

Use `MetricDelta` when one business operation changes several lifecycle
counters. The fields are additive deltas, never absolute target values, and the
whole delta is validated and committed at one linearization point:

```rust
let completed = 10;
let succeeded = 8;
items.apply_delta(
    MetricDelta::new()
        .started(completed)
        .succeeded(succeeded)
        .unclassified(completed - succeeded),
)?;
```

Concurrent additive deltas cannot overwrite one another. A delta that tries to
finish more active work than exists at its linearization point returns
`MetricError::InsufficientActive` and changes nothing. If a consumer needs an
authoritative aggregate, use a terminal event or synchronize the business
update and report boundary outside `MetricHandle`.

The lifecycle remains one-way: `started` adds active work, and each terminal
field consumes active work into exactly one terminal outcome. Within one delta,
`started` is accounted for before its terminal fields, so an atomic
start-and-finish batch is valid. Across concurrent deltas, serialization order
is significant: a terminal-only delta may be rejected if the corresponding
`started` delta has not linearized yet; retry it or coordinate the two updates
when that ordering matters.

`completed()` includes unclassified completion. Use
`MetricSnapshot::unclassified()` to inspect the neutral terminal outcome
explicitly.

## Correlate events with operation attributes

Set optional string attributes on `ProgressBuilder` before `start()`:

```rust
let progress = Progress::builder(&reporter)
    .attribute("tenant_id", "tenant-7")
    .attribute("trace_id", "trace-abc")
    .metric(Metric::new("files", "Files"))
    .start()?;
```

Attributes are immutable after startup and are attached to every event. They
are ordered for stable text and JSON output. Do not put secrets or tokens in
attributes because reporters emit them as supplied.

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
            if let Err(report_error) = progress.fail() {
                eprintln!("terminal reporting also failed: {report_error}");
            }
            Err(work_error)
        }
    }
}
```

Use `cancel()` instead when the caller or user stops the operation. The example
retains the original work error and records a separate terminal reporting
error.

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

## Writer buffering and partial failures

The built-in text and JSON Lines reporters pass each complete record to the
configured `Write` implementation. A successful `report()` means that writer
accepted the record; it does not call `flush()`. `into_inner()` only returns
the writer. After reporting has stopped and the writer is no longer shared,
flush it explicitly and handle the result:

```rust
use std::io::{BufWriter, Write};
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(BufWriter::new(Vec::<u8>::new()));
let mut progress = Progress::builder(&reporter)
    .metric(Metric::new("tasks", "Tasks").total(1))
    .start()?;
let tasks = progress.metric("tasks").expect("configured metric must exist");
tasks.start(1)?;
tasks.succeed(1)?;
progress.report()?;
progress.finish()?;
let mut writer = reporter.into_inner()?;
writer.flush()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

For a `File`, handle `flush()` errors the same way. A successful flush is not a
disk-persistence guarantee; applications that require persistence must also
use the appropriate file synchronization and transaction policy. A failed
`write_all` may already have written a prefix, which is not rolled back, and
later records are not guaranteed to start on a fresh line. Reopen or replace
the target, or resynchronize using the storage format's recovery procedure,
before continuing. The reporters do not retry automatically.

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

thread::scope(|scope| -> Result<(), qubit_progress::AutoReporterError> {
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

For a zero interval, `notify()` coalesces repeated calls into at most one pending report. For a positive interval, the worker emits heartbeats at the minimum interval and `notify()` is a no-op, avoiding synchronization work for worker threads. `notify()` is harmless after `stop()`. Always call `stop()` and handle its result before leaving the scope. `Progress` remains exclusively borrowed for the entire `thread::scope`; `stop()` joins the worker but does not end that borrow early. Wait until the scope returns before reporting manually, changing stages, or consuming the operation. Use an inner scope if the surrounding work must continue afterward.

The automatic driver uses one scoped `std::thread` per enabled `Progress`
operation and does not provide an automatic driver for `AsyncProgress`. With an
`AsyncReporter`, schedule and await `AsyncProgress::report_if_due_async()` on
the application's runtime. With a synchronous `Reporter`, use
`Progress::report_if_due()`; its callback executes synchronously, so the
application must keep blocking sinks off the runtime's worker threads. The
application owns scheduling, cancellation, and backpressure policies.

`AutoReporterStatus::is_failed()` becomes true when the background reporter exits with an
error or panic. Workers can observe a cloned `AutoReporterStatus` to stop expensive work
early, but `stop()` remains authoritative: it joins the thread, returns its
validation or reporter error, or a structured worker panic. Dropping the handle
still stops and joins the worker without propagating a panic. Both `stop()` and
dropped handles synchronously wait for the worker to exit, with no shutdown
timeout. The stop signal wakes the reporting loop but cannot interrupt a
reporter callback already in progress. In notification-driven mode, shutdown
may also deliver a pending coalesced report. A callback that does not return can
keep shutdown and scope exit waiting indefinitely. Applications that require
bounded shutdown must bound the sink's blocking operations or provide
cooperative cancellation within the sink.

## Asynchronous reporters

`AsyncReporter` returns a boxed, sendable future from `report()`, and
`AsyncProgress` mirrors the synchronous lifecycle with `start_async()`,
`report_async()`, due reports, and async terminal methods. The crate does not
depend on a particular async runtime; the caller awaits these methods on its
own executor. A completed `Ok(())` means the reporter accepted the event.

The reporter method returns the boxed future type `ReportFuture<'a>`:

```rust
impl AsyncReporter for RecordingSink {
    fn report<'a>(&'a self, event: &'a Event) -> ReportFuture<'a> {
        Box::pin(async move {
            self.0.lock().expect("recording lock must work").push(event.clone());
            Ok(())
        })
    }
}
```

The operation awaits startup, records work synchronously, then awaits due and
terminal reports. On business failure it tries the failed terminal event while
preserving the original work error:

```rust
async fn run_batch(reporter: &dyn AsyncReporter, fail_work: bool) -> Result<(), Box<dyn Error>> {
    let mut progress = AsyncProgress::builder(reporter)
        .metric(Metric::new("items", "Items").total(1))
        .start_async()
        .await?;
    let items = progress.metric("items").expect("configured metric must exist");
    items.start(1)?;

    if fail_work {
        let work_error = io::Error::other("demonstration work failure");
        items.fail(1)?;
        if let Err(report_error) = progress.fail_async().await {
            eprintln!("terminal reporting also failed: {report_error}");
        }
        return Err(Box::new(work_error));
    }

    items.succeed(1)?;
    progress.report_if_due_async().await?;
    progress.finish_async().await?;
    Ok(())
}
```

If a caller cancels an in-flight report future, delivery status is unknown.
Running reports consume their sequence before awaiting delivery. Calling a
consuming async terminal method moves the operation into its future immediately;
polling drives elapsed-time sampling, metric freezing, validation where required,
and delivery when enabled. Dropping an unpolled terminal future closes the local
operation and its metric handles without sending a terminal event. Do not retry
a terminal event automatically unless the reporter provides deduplication.

Startup has a separate cancellation boundary. Dropping an unpolled
`start_async()` future does not invoke the reporter. Cancelling it while Started
delivery is pending drops the internal operation: no `AsyncProgress`, `StartError`,
or `DeliveryError` is returned, no retry occurs, and no terminal event is generated.
The sink may already have accepted Started. Starting again creates a new
operation identity, not a replay of the original event. Applications that require
remote reconciliation must use their own correlation and recovery policy.

The complete standalone example at [examples/async_reporting.rs](../examples/async_reporting.rs)
implements an in-memory `RecordingSink`, then awaits startup, due reporting,
and successful completion. Run it with:

```sh
cargo run --locked --example async_reporting
```

It prints:

```text
success: Started -> Running -> Succeeded; work error: Started -> Failed
```

The example's default interval is zero, so `report_if_due_async().await`
reports immediately. With a positive interval, call
`AsyncProgress::report_if_due_async().await` from the application's runtime
schedule. The synchronous counterpart is `Progress::report_if_due()` and does
not use `.await`. `RecordingSink` only records events in memory; it is not a
persistent store. A real sink should await its storage operation and map its
error with `ReporterError::new`, allowing successful-path errors to propagate
through the `?` operators. The business-failure path retains and returns the
business error while recording a separate terminal reporting error. Ordinary
early returns do not automatically emit a terminal event; arrange `fail_async()`
or `cancel_async()` at the point where the outcome is known. Metric validation
failures and delivery failures are distinct: completion validation can fail
before terminal delivery is attempted, while a terminal delivery error means
the operation was consumed and its delivery was attempted.

For `report_if_due_async()`, a polled due `Running` future consumes its
sequence. If it completes with success or error, the next deadline is measured
from completion. If it remains pending and is cancelled, the deadline is left
unchanged, so the next call may be due immediately; the operation remains
usable. Dropping an unpolled future consumes neither a sequence nor a deadline.
Disabled operations emit nothing and keep their scheduling state unchanged.
Applications that cancel on timeout and need to limit retries must apply their
own backoff; the crate does not retry automatically.

| Outcome | Sequence | Next deadline | Operation |
| --- | --- | --- | --- |
| Not due or disabled | Unchanged | Unchanged | Usable |
| Due report completes, with success or error | Consumed | Reset from completion | Usable |
| Polled due report is cancelled while pending | Consumed | Unchanged; next call may be due | Usable |
| Unpolled `Running` future is dropped | Unchanged | Unchanged | Usable |

## Disabled operations

Enablement belongs to `Reporter::is_enabled()` and is sampled once by `start()`. A disabled operation still validates its fixed configuration and maintains metric state, but emits no events and starts no automatic reporting thread. This makes an unconditional reporting path cheap when a sink is disabled.

`NoopReporter` is useful when the caller needs a reporter that explicitly disables output. A custom reporter can override `is_enabled()` to connect enablement to application configuration.

## Reporters and structured output

Implement `Reporter` to consume `&Event`; implementations must be `Send + Sync`. Built-in reporters are:

- `TextReporter<W>` writes one human-readable line per event to any `Write + Send` target.
- `NoopReporter` disables events.
- `JsonLinesReporter<W>` is available with `json-lines` and writes one complete JSON event per line.
- `LogReporter` is available with `log` and writes the event's `Debug` representation at info level.

Text output keeps printable Unicode, including ordinary Chinese text, readable.
It escapes control characters such as line feed, carriage return, and tab in
metadata, preserving one output line per event.

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
the delivery attempt failed. `finish()` returns a static `FinishError`, so it
can be propagated with `?`. `elapsed()` returns the elapsed operation time
sampled at the beginning of the terminal attempt, not the duration of the
`finish()` call. The sample precedes freezing metric updates, completion
validation where applicable, and terminal delivery; subsequent waiting is
excluded. Async methods take the sample when the future is polled and reaches
terminal logic, not when the future is created. Use it to inspect the sample
and its `CompletionError` or `TerminalError`. Use `finish_recoverable()` and
`RecoverableFinishError` only when repair and retry are required.

Terminal methods return these results:

| Method | Return value |
| --- | --- |
| `finish` | `Result<Duration, FinishError>` |
| `finish_recoverable` | `Result<Duration, RecoverableFinishError<'reporter>>` |
| `finish_unchecked`, `fail`, `cancel` | `Result<Duration, TerminalError>` |
| `finish_async` | after `.await`: `Result<Duration, FinishError>` |
| `finish_unchecked_async`, `fail_async`, `cancel_async` | after `.await`: `Result<Duration, TerminalError>` |

`FinishError` preserves the distinction between completion validation and
terminal delivery; `TerminalError` retains elapsed time if final delivery
fails. For automatic reporting, `AutoReporter::stop()` returns
`AutoReporterError`, whose `Emission` variant carries a normal delivery failure
and whose `Panicked` variant preserves the worker panic payload. `Drop` remains
panic-free and is intended for best-effort cleanup.

When the `serde` feature is enabled, deserializing `Stage`, `MetricSnapshot`, or `Event` validates the same metadata and count invariants used by live progress operations. Treat a deserialization error as invalid external progress data rather than a recoverable event state.

## Further reference

See the generated API documentation on [docs.rs](https://docs.rs/qubit-progress) for every type, error variant, reporter feature, and serialization detail.
