# Qubit Progress

`qubit-progress` 为 Qubit Rust 库和应用提供轻量级进度汇报抽象。

它把进度建模为不可变、自描述的事件。每个事件包含：

- `ProgressSchema`：当前逻辑操作的 metric 定义。
- `ProgressMetric`：稳定的 metric `id` 和适合展示的 `name`。
- `ProgressCounter`：某个 metric 的 `u64` 计数器。
- `ProgressPhase`：`started`、`running`、`finished`、`failed`、`canceled`。
- `ProgressStage`：可选的多阶段操作信息。
- `elapsed`：`std::time::Duration`，通过 `qubit-serde` 序列化为 `110ms` 这类字符串。

一个 `Progress` 对应一个逻辑操作。不要把多个无关操作混进同一个 reporter 事件流，除非该 reporter 明确支持复用和分流。多线程任务应先聚合 counters，再通过这个操作级 `Progress` 统一汇报。

## 安装

```toml
[dependencies]
qubit-progress = "0.5"
```

## 构造自描述事件

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressEvent,
    ProgressMetric,
    ProgressPhase,
    ProgressSchema,
};

let schema = ProgressSchema::new(vec![
    ProgressMetric::new("entries", "Entries"),
    ProgressMetric::new("bytes", "Bytes"),
]);

let event = ProgressEvent::builder(schema)
    .running()
    .stage_named("copy", "Copy files")
    .counter("entries", |counter| counter.total(10).completed(4))
    .counter("bytes", |counter| counter.total(1_000_000).completed(400_000))
    .elapsed(Duration::from_millis(110))
    .build();

assert_eq!(event.phase(), ProgressPhase::Running);
assert_eq!(event.counter("entries").map(|c| c.completed_count()), Some(4));
```

## 汇报一个操作

```rust
use std::time::Duration;

use qubit_progress::{
    Progress,
    ProgressMetric,
    ProgressSchema,
    WriterProgressReporter,
};

let schema = ProgressSchema::new(vec![
    ProgressMetric::new("entries", "Entries"),
    ProgressMetric::new("bytes", "Bytes"),
]);
let reporter = WriterProgressReporter::from_writer(std::io::stdout());
let mut progress = Progress::new(&reporter, Duration::from_secs(1), schema);

progress.report_started(|event| event.counter("entries", |counter| counter.total(3)));

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

`report_running_if_due` 只有在达到汇报间隔时才会调用 builder 闭包，因此正数间隔下的热路径开销很低。

## 后台汇报线程

当 worker 线程更新业务状态，而协调线程需要周期性发出 `running` 事件时，可以使用 `Progress::spawn_running_reporter`。worker 更新共享状态后调用 `RunningProgressPointHandle::report()`；当间隔为 `Duration::ZERO` 时，这个调用会唤醒后台汇报线程。
`RunningProgressGuard` 持有这个后台汇报线程，`RunningProgressPointHandle` 是可克隆的 worker 侧唤醒句柄。

```rust
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use qubit_progress::{
    NoOpProgressReporter,
    Progress,
    ProgressCounter,
    ProgressSchema,
};

let reporter = NoOpProgressReporter;
let completed = Arc::new(AtomicU64::new(0));
let progress = Progress::new(
    &reporter,
    Duration::ZERO,
    ProgressSchema::single("entries", "Entries"),
);

thread::scope(|scope| {
    let snapshot_completed = Arc::clone(&completed);
    let running = progress.spawn_running_reporter(scope, move || {
        vec![ProgressCounter::new("entries")
            .total(3)
            .completed(snapshot_completed.load(Ordering::Acquire))]
    });
    let point = running.point_handle();

    completed.store(1, Ordering::Release);
    assert!(point.report());

    running.stop_and_join();
});
```

## JSON 序列化

进度事件支持 serde 序列化。`elapsed` 使用 `qubit-serde` 的 `duration_with_unit` 适配器，因此 JSON 更紧凑，也更适合 agent 读取。

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressEvent,
    ProgressMetric,
    ProgressSchema,
};

let schema = ProgressSchema::new(vec![ProgressMetric::new("entries", "Entries")]);
let event = ProgressEvent::builder(schema)
    .running()
    .counter("entries", |counter| counter.total(5).completed(2))
    .elapsed(Duration::from_millis(110))
    .build();

let json = serde_json::to_string(&event).expect("event should serialize");
assert!(json.contains("\"elapsed\":\"110ms\""));
```

## 内置 reporter

- `NoOpProgressReporter`：忽略事件。
- `WriterProgressReporter`：把人类可读文本写入任意 `Write` sink。
- `StdoutProgressReporter`：写入 stdout。
- `StderrProgressReporter`：写入 stderr。
- `LoggerProgressReporter`：通过 `log` crate 输出。

Reporter 接收事件的接口是：

```rust
fn report(&self, event: &ProgressEvent);
```

Reporter 可以按 `metric_id` 分组，并通过 `event.schema()` 解析适合展示的 metric 名称。
