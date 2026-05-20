# Qubit Progress User Guide

`qubit-progress` is a small progress reporting layer for Rust libraries and
applications. It does not own your work state. Instead, your code keeps domain
state, converts it into progress counters, and sends immutable progress events
to a reporter.

This guide explains how to use the crate, how to choose the right API, and how
to extend it with custom reporters such as GUI progress bars or JSON-lines
reporters.

## Installation

```toml
[dependencies]
qubit-progress = "0.5"
```

If you want to serialize progress events yourself, also add `serde_json` or
another serde-compatible format crate:

```toml
[dependencies]
serde_json = "1"
```

If you use the consumer-based extension examples directly, add `qubit-function`:

```toml
[dependencies]
qubit-function = "0.15"
```

Some concurrent examples in this guide use `qubit-atomic` to avoid explicit
standard-library memory ordering parameters:

```toml
[dependencies]
qubit-atomic = "0.13"
```

## Core Model

A progress stream has five main concepts:

| Concept | Type | Meaning |
| --- | --- | --- |
| Schema | `ProgressSchema` | The metric dictionary for one operation. |
| Metric | `ProgressMetric` | A stable metric id and a display name. |
| Counter | `ProgressCounter` | The current numbers for one metric. |
| Event | `ProgressEvent` | One immutable progress snapshot. |
| Metric snapshot | `ProgressMetricSnapshot` | One event counter flattened with metric metadata and event context. |
| Reporter | `ProgressReporter` | A sink that receives events. |

A `Progress` value ties these concepts together for one logical operation. It
keeps the operation start time, the report interval, the optional stage, and the
reporter reference.

The event is self-describing: it carries its schema. This makes serialized JSON
usable by logs, databases, agents, and external consumers without requiring a
separate schema registry.

When a reporter wants to handle one metric at a time, it can call
`ProgressEvent::metric_snapshots()`. Each `ProgressMetricSnapshot` contains the
complete `ProgressMetric`, event phase, optional stage, flattened counter
values, and elapsed time.

## Quick Start

The common workflow is:

1. Define a schema.
2. Choose a reporter.
3. Create a `Progress` run.
4. Report `started`, `running`, and terminal events.

```rust
use std::time::Duration;

use qubit_progress::{
    Progress,
    ProgressMetric,
    ProgressSchema,
    StderrProgressReporter,
};

let schema = ProgressSchema::new(vec![
    ProgressMetric::new("entries", "Entries"),
    ProgressMetric::new("bytes", "Bytes"),
]);
let reporter = StderrProgressReporter::new();
let mut progress = Progress::new(&reporter, Duration::from_secs(1), schema);

progress.report_started(|event| {
    event
        .counter("entries", |counter| counter.total(3))
        .counter("bytes", |counter| counter.total(1_024))
});

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

## Defining Metrics and Schemas

Use one metric for each unit that should be tracked independently. Common
metric ids are `entries`, `files`, `dirs`, `bytes`, `objects`, `requests`, and
`tasks`.

```rust
use qubit_progress::{
    ProgressMetric,
    ProgressSchema,
};

let schema = ProgressSchema::new(vec![
    ProgressMetric::new("files", "Files"),
    ProgressMetric::new("bytes", "Bytes"),
]);

assert_eq!(schema.metric_name("files"), Some("Files"));
assert!(schema.contains_metric("bytes"));
```

For a single-metric operation, use `ProgressSchema::single` or
`Progress::single_metric`:

```rust
use std::time::Duration;

use qubit_progress::{
    NoOpProgressReporter,
    Progress,
    ProgressSchema,
};

let schema = ProgressSchema::single("entries", "Entries");
assert_eq!(schema.metric_name("entries"), Some("Entries"));

let reporter = NoOpProgressReporter;
let progress = Progress::single_metric(&reporter, Duration::from_secs(1), "entries", "Entries");
assert_eq!(progress.schema().metric_name("entries"), Some("Entries"));
```

`ProgressSchema::validate_counter` and `ProgressSchema::validate_counters` are
lightweight checks. They verify that counter metric ids exist in the schema and
that an event does not contain duplicate metric ids. They intentionally do not
validate numeric relationships such as `completed <= total`, because retry,
compensation, and dynamically discovered totals are domain-specific.

## Counters

`ProgressCounter` stores `u64` values. This keeps serialized output stable and
avoids platform-dependent `usize` widths.

```rust
use qubit_progress::ProgressCounter;

let counter = ProgressCounter::new("bytes")
    .total(10_000)
    .completed(4_000)
    .active(1)
    .succeeded(3_999)
    .failed(1);

assert_eq!(counter.total_count(), Some(10_000));
assert_eq!(counter.completed_count(), 4_000);
assert_eq!(counter.remaining_count(), Some(5_999));
assert_eq!(counter.progress_percent(), Some(40.0));
```

Use `unknown_total` or omit `total` when the operation is open-ended:

```rust
use qubit_progress::ProgressCounter;

let counter = ProgressCounter::new("records").completed(12);
assert_eq!(counter.total_count(), None);
assert_eq!(counter.progress_percent(), None);
```

## Building Events Directly

If you only need a data model and do not need `Progress` to track elapsed time
or report intervals, build events directly:

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressEvent,
    ProgressMetric,
    ProgressPhase,
    ProgressSchema,
};

let schema = ProgressSchema::new(vec![ProgressMetric::new("entries", "Entries")]);
let event = ProgressEvent::builder(schema)
    .running()
    .counter("entries", |counter| counter.total(10).completed(4))
    .elapsed(Duration::from_millis(250))
    .build();

assert_eq!(event.phase(), ProgressPhase::Running);
assert_eq!(event.counter("entries").map(|counter| counter.completed_count()), Some(4));
```

There are also direct constructors for lifecycle phases:

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressCounter,
    ProgressEvent,
    ProgressSchema,
};

let event = ProgressEvent::finished(
    ProgressSchema::single("entries", "Entries"),
    vec![ProgressCounter::new("entries").total(3).completed(3)],
    Duration::from_secs(2),
);

assert!(event.phase().is_terminal());
```

## Lifecycle Phases

`ProgressPhase` has five values:

| Phase | Meaning |
| --- | --- |
| `Started` | The operation has started. |
| `Running` | The operation is still running. |
| `Finished` | The operation completed successfully. |
| `Failed` | The operation failed. |
| `Canceled` | The operation was canceled. |

`Finished`, `Failed`, and `Canceled` are terminal phases.

```rust
use qubit_progress::ProgressPhase;

assert_eq!(ProgressPhase::Running.as_str(), "running");
assert!(ProgressPhase::Failed.is_terminal());
```

## Stages

Use `ProgressStage` for multi-step operations such as scan, copy, verify, and
publish. Stage metadata is optional. It can include an id, display name, index,
total stage count, and relative weight.

```rust
use std::time::Duration;

use qubit_progress::{
    NoOpProgressReporter,
    Progress,
    ProgressSchema,
    ProgressStage,
};

let reporter = NoOpProgressReporter;
let progress = Progress::new(
    &reporter,
    Duration::from_secs(1),
    ProgressSchema::single("files", "Files"),
)
.with_stage(
    ProgressStage::new("copy", "Copy files")
        .with_index(1)
        .with_total_stages(3)
        .with_weight(0.7),
);

let event = progress.report_started(|event| event.counter("files", |counter| counter.total(10)));
assert_eq!(event.stage().map(|stage| stage.id()), Some("copy"));
```

A stage can be attached to the `Progress` run or overridden per event through
the event builder.

## Reporting Cadence

Use `report_running` when you want to emit immediately. Use
`report_running_if_due` on hot paths where repeated reporting should be
throttled.

```rust
use std::time::Duration;

use qubit_progress::{
    NoOpProgressReporter,
    Progress,
    ProgressSchema,
};

let reporter = NoOpProgressReporter;
let mut progress = Progress::new(
    &reporter,
    Duration::from_secs(60),
    ProgressSchema::single("entries", "Entries"),
);

let not_due = progress.report_running_if_due(|event| {
    event.counter("entries", |counter| counter.total(10).completed(1))
});
assert!(not_due.is_none());

let emitted = progress.report_running(|event| {
    event.counter("entries", |counter| counter.total(10).completed(1))
});
assert_eq!(emitted.counter("entries").map(|counter| counter.completed_count()), Some(1));
```

When the interval is `Duration::ZERO`, every `report_running_if_due` call is due.

## Background Running Reporter

Use `Progress::spawn_running_reporter` when worker threads update shared domain
state and a coordinator should emit periodic `running` events.

The second argument is the `snapshot` closure:

```rust
FnMut() -> Vec<ProgressCounter>
```

It must return the complete current counter snapshot for the next `running`
event. It does not return a delta, a percentage, or a `ProgressEvent`.
`Progress` will wrap the returned counters with the current schema, stage,
elapsed time, and `ProgressPhase::Running`.

The returned counters should normally use metric ids declared in the
`ProgressSchema` used to create the `Progress` run.

```rust
// Good: full current state.
vec![ProgressCounter::new("entries").total(100).completed(42)]

// Not the intended meaning: "one more item since last report".
vec![ProgressCounter::new("entries").completed(1)]
```

The snapshot closure is called by the background reporter thread when a running
event may be due.

```rust
use std::{
    thread,
    time::Duration,
};

use qubit_atomic::ArcAtomic;
use qubit_progress::{
    NoOpProgressReporter,
    Progress,
    ProgressCounter,
    ProgressSchema,
};

let reporter = NoOpProgressReporter;
let completed = ArcAtomic::new(0u64);
let progress = Progress::new(
    &reporter,
    Duration::ZERO,
    ProgressSchema::single("entries", "Entries"),
);

thread::scope(|scope| {
    let snapshot_completed = completed.clone();
    let running = progress.spawn_running_reporter(scope, move || {
        // Return a full current snapshot for the running event.
        // This says "3 total, currently 1 completed", not "completed 1 more".
        vec![ProgressCounter::new("entries")
            .total(3)
            .completed(snapshot_completed.load())]
    });
    let point = running.point_handle();

    let worker = scope.spawn({
        let completed = completed.clone();
        let point = point.clone();
        move || {
            completed.fetch_add(1);
            assert!(point.report());
        }
    });
    worker.join().expect("worker should complete");

    running.stop_and_join();
});
```

Important rules:

- Always call `stop_and_join` before the thread scope exits.
- Worker handles can only report points; they cannot stop the loop.
- With a zero interval, `RunningProgressPointHandle::report` wakes the reporter loop.
- With a positive interval, worker point reporting is a cheap no-op and the loop wakes on timeout.
- Panics from the reporter thread are propagated by `stop_and_join`.

## Built-in Reporters

| Reporter | Use case |
| --- | --- |
| `NoOpProgressReporter` | Tests, optional progress, or disabled reporting. |
| `MetricSnapshotProgressReporter` | Send structured `ProgressMetricSnapshot` objects to a consumer. |
| `FormattedProgressReporter` | Format each metric snapshot and send strings to a consumer. |
| `HumanReadableProgressReporter` | Send human-readable metric snapshot strings to a consumer. |
| `JsonProgressReporter` | Send JSON metric snapshot strings to a consumer. |
| `WriterProgressReporter<W>` | Write human-readable metric snapshot lines to any `Write + Send` sink. |
| `StdoutProgressReporter` | Command-line progress to stdout. |
| `StderrProgressReporter` | Command-line progress to stderr. |
| `LoggerProgressReporter` | Emit progress through the `log` crate. |
| `JsonWriterProgressReporter<W>` | Write JSON metric snapshot lines to any `Write + Send` sink. |
| `JsonStdoutProgressReporter` | Command-line JSON progress to stdout. |
| `JsonStderrProgressReporter` | Command-line JSON progress to stderr. |
| `JsonLoggerProgressReporter` | Emit JSON metric snapshots through the `log` crate. |

Example with an in-memory writer:

```rust
use std::{
    io::Cursor,
    sync::{Arc, Mutex},
    time::Duration,
};

use qubit_progress::{
    Progress,
    ProgressSchema,
    WriterProgressReporter,
};

let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
let reporter = WriterProgressReporter::new(output.clone());
let progress = Progress::new(
    &reporter,
    Duration::ZERO,
    ProgressSchema::single("entries", "Entries"),
);

progress.report_finished(|event| {
    event.counter("entries", |counter| counter.total(2).completed(2).succeeded(2))
});

let text = String::from_utf8(
    output.lock().expect("output should lock").get_ref().clone(),
)
.expect("progress output should be UTF-8");
assert!(text.contains("finished"));
assert!(text.contains("Entries 2/2"));
```

## JSON Serialization

`ProgressEvent`, `ProgressSchema`, `ProgressMetric`, `ProgressCounter`,
`ProgressPhase`, and `ProgressStage` are serde-serializable. The `elapsed`
field uses `qubit-serde` duration strings such as `110ms`.

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressEvent,
    ProgressMetric,
    ProgressSchema,
};

let event = ProgressEvent::builder(ProgressSchema::new(vec![
    ProgressMetric::new("entries", "Entries"),
]))
.running()
.counter("entries", |counter| counter.total(5).completed(2))
.elapsed(Duration::from_millis(110))
.build();

let json = serde_json::to_string(&event).expect("event should serialize");
assert_eq!(
    json,
    concat!(
        r#"{"schema":{"metrics":["#,
        r#"{"id":"entries","name":"Entries"}"#,
        r#"]},"phase":"running","counters":["#,
        r#"{"metric_id":"entries","total_count":5,"completed_count":2,"#,
        r#""active_count":0,"succeeded_count":0,"failed_count":0}"#,
        r#"],"elapsed":"110ms"}"#,
    ),
);
```

This representation is useful for agent-readable logs because each event
contains both the metric ids and their display names.

For line-oriented structured output, prefer the built-in JSON metric snapshot
reporters. They write one JSON object per metric counter:

```rust
use std::{
    io::Cursor,
    sync::{Arc, Mutex},
    time::Duration,
};

use qubit_progress::{
    JsonWriterProgressReporter,
    ProgressCounter,
    ProgressEvent,
    ProgressReporter,
    ProgressSchema,
};

let output = Arc::new(Mutex::new(Cursor::new(Vec::new())));
let reporter = JsonWriterProgressReporter::new(output.clone());
reporter.report(&ProgressEvent::running(
    ProgressSchema::single("entries", "Entries"),
    vec![ProgressCounter::new("entries").total(5).completed(2)],
    Duration::from_millis(110),
));

let text = String::from_utf8(
    output.lock().expect("output should lock").get_ref().clone(),
)
.expect("JSON output should be UTF-8");
assert!(text.contains(r#""metric":{"id":"entries","name":"Entries"}"#));
assert!(text.contains(r#""elapsed":"110ms""#));
```

Use `JsonProgressReporter` when you already have a
`qubit_function::Consumer<String>`, `JsonWriterProgressReporter` for any
`Write` sink, and `JsonLoggerProgressReporter` for `log` output.

## Consuming Metric Snapshots Directly

Some integrations should not format progress as strings at all. GUI progress
bars, metrics collectors, and database writers often want structured objects.
Use `MetricSnapshotProgressReporter` for those cases:

```rust
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use qubit_function::ArcConsumer;
use qubit_progress::{
    MetricSnapshotProgressReporter,
    ProgressCounter,
    ProgressEvent,
    ProgressMetricSnapshot,
    ProgressReporter,
    ProgressSchema,
};

let snapshots = Arc::new(Mutex::new(Vec::<ProgressMetricSnapshot>::new()));
let captured = Arc::clone(&snapshots);
let consumer = ArcConsumer::new(move |snapshot: &ProgressMetricSnapshot| {
    captured
        .lock()
        .expect("snapshot list should lock")
        .push(snapshot.clone());
});
let reporter = MetricSnapshotProgressReporter::new(consumer);

reporter.report(&ProgressEvent::running(
    ProgressSchema::single("entries", "Entries"),
    vec![ProgressCounter::new("entries").total(5).completed(2)],
    Duration::from_millis(110),
));

let snapshots = snapshots.lock().expect("snapshot list should lock");
assert_eq!(snapshots[0].metric_id(), "entries");
assert_eq!(snapshots[0].completed_count(), 2);
```

## Implementing a Custom Reporter

A reporter implements one trait:

```rust
use qubit_progress::{
    ProgressEvent,
    ProgressReporter,
};

struct MyReporter;

impl ProgressReporter for MyReporter {
    fn report(&self, event: &ProgressEvent) {
        println!("phase={}", event.phase());
    }
}
```

`ProgressReporter` requires `Send + Sync`, so reporters can be shared between
worker and reporter threads. If your reporter stores mutable state, use a
thread-safe primitive such as `Mutex`, `RwLock`, an atomic type, or a channel.

### Recording Reporter for Tests

```rust
use std::sync::Mutex;

use qubit_progress::{
    ProgressEvent,
    ProgressReporter,
};

#[derive(Default)]
struct RecordingReporter {
    events: Mutex<Vec<ProgressEvent>>,
}

impl RecordingReporter {
    fn events(&self) -> Vec<ProgressEvent> {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ProgressReporter for RecordingReporter {
    fn report(&self, event: &ProgressEvent) {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event.clone());
    }
}
```

This style is useful for tests because `ProgressEvent` is immutable and cloneable.

### JSON Lines Reporter

A JSON-lines reporter is often the best format for agents, batch jobs, and
external monitoring processes.

```rust
use std::{
    io::Write,
    sync::Mutex,
};

use qubit_progress::{
    ProgressEvent,
    ProgressReporter,
};

struct JsonLinesProgressReporter<W> {
    writer: Mutex<W>,
}

impl<W> JsonLinesProgressReporter<W> {
    fn new(writer: W) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }
}

impl<W> ProgressReporter for JsonLinesProgressReporter<W>
where
    W: Write + Send,
{
    fn report(&self, event: &ProgressEvent) {
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        serde_json::to_writer(&mut *writer, event).expect("progress event should serialize");
        writeln!(writer).expect("progress event should write");
    }
}
```

For production code, decide explicitly whether reporter I/O failures should
panic, be ignored, or be sent to a fallback error channel.

## GUI Progress Bar Reporter

GUI toolkits usually require UI updates to happen on the UI thread. A reporter
should therefore avoid touching widgets directly from `report`. Instead, send a
small message to the UI thread and let the UI thread update the progress bar.

The following example uses `std::sync::mpsc` to model that boundary:

```rust
use std::sync::mpsc::Sender;

use qubit_progress::{
    ProgressEvent,
    ProgressPhase,
    ProgressReporter,
};

#[derive(Debug, Clone)]
struct GuiProgressMessage {
    phase: ProgressPhase,
    stage_name: Option<String>,
    completed: u64,
    total: Option<u64>,
    percent: Option<f64>,
}

#[derive(Clone)]
struct GuiProgressReporter {
    sender: Sender<GuiProgressMessage>,
    primary_metric_id: String,
}

impl GuiProgressReporter {
    fn new(sender: Sender<GuiProgressMessage>, primary_metric_id: &str) -> Self {
        Self {
            sender,
            primary_metric_id: primary_metric_id.to_owned(),
        }
    }
}

impl ProgressReporter for GuiProgressReporter {
    fn report(&self, event: &ProgressEvent) {
        let Some(counter) = event.counter(&self.primary_metric_id) else {
            return;
        };
        let message = GuiProgressMessage {
            phase: event.phase(),
            stage_name: event.stage().map(|stage| stage.name().to_owned()),
            completed: counter.completed_count(),
            total: counter.total_count(),
            percent: counter.progress_percent(),
        };
        let _ = self.sender.send(message);
    }
}
```

The UI side can map the message to widget state:

```rust
use std::sync::mpsc::Receiver;

fn handle_gui_messages(receiver: Receiver<GuiProgressMessage>) {
    while let Ok(message) = receiver.recv() {
        match message.total {
            Some(total) => {
                println!(
                    "set progress bar to {}/{} ({:?}%)",
                    message.completed,
                    total,
                    message.percent,
                );
            }
            None => {
                println!("show indeterminate progress: {} completed", message.completed);
            }
        }

        if message.phase.is_terminal() {
            println!("enable close button");
            break;
        }
    }
}
```

For a real GUI crate, replace the `println!` calls with toolkit-specific UI
updates. Keep the same architecture: reporter sends messages, UI thread owns
widgets.

## Extension Guidelines

When implementing a reporter:

- Keep `report` short. Expensive work should be offloaded to another thread or queue.
- Do not block worker hot paths unless progress reporting is part of the operation contract.
- Treat one reporter stream as one logical operation unless you explicitly add multiplexing.
- Use `event.schema()` to resolve display names instead of hard-coding metric labels.
- Use `event.counter("metric_id")` for a primary metric and `event.counters()` for multi-metric displays.
- Decide the failure policy: panic, ignore, log, or send error messages elsewhere.
- Avoid holding locks while calling external code.
- Prefer sending compact messages to GUI or async runtimes instead of storing UI handles inside the reporter.

When designing metric ids:

- Use stable lowercase ids such as `entries`, `bytes`, `files`, or `requests`.
- Keep ids machine-readable; use `ProgressMetric::name` for display labels.
- Use separate metrics for separate units. Do not mix bytes and files in one counter.
- Keep the same schema for the whole operation.

When reporting counters:

- Use known totals when available.
- Use unknown totals for streams, discovery phases, and open-ended work.
- Use `active` for currently in-flight units.
- Use `succeeded` and `failed` when callers need final outcome counts.
- Keep domain state outside `Progress`; build fresh counters when reporting.

## Choosing an API

| Need | Recommended API |
| --- | --- |
| Build one standalone progress payload | `ProgressEvent::builder` |
| Track elapsed time for one operation | `Progress` |
| Emit periodic running updates from a loop | `report_running_if_due` |
| Emit immediate running updates | `report_running` |
| Worker threads update state and a coordinator reports | `spawn_running_reporter` |
| Disable progress | `NoOpProgressReporter` |
| Human-readable CLI output | `StdoutProgressReporter` or `StderrProgressReporter` |
| Structured logs | custom JSON-lines reporter |
| GUI progress bar | custom channel-based reporter |

## Crate Boundary

`qubit-progress` intentionally stays small. It provides:

- progress data models;
- operation-scoped lifecycle helpers;
- background running-report helpers;
- reporter traits and a few built-in reporters.

It does not provide:

- terminal UI widgets;
- GUI toolkit integration;
- async runtime integration;
- task scheduling;
- tracing infrastructure;
- metrics databases or dashboards.

Those integrations should live in downstream crates or applications by
implementing `ProgressReporter`.
