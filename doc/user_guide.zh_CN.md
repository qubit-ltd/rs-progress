# Qubit Progress 用户指南

`qubit-progress` 是一个面向 Rust 库和应用的小型进度汇报层。它不持有你的业务状态。你的代码负责保存业务状态，并在需要汇报时把业务状态转换成 progress counters，再发送不可变的 progress events 给 reporter。

本指南说明如何使用本库，如何选择合适 API，以及如何扩展实现自己的 reporter，例如 GUI 进度条 reporter 或 JSON-lines reporter。

## 安装

```toml
[dependencies]
qubit-progress = "0.5"
```

如果你需要自己序列化 progress event，可以同时加入 `serde_json` 或其他 serde 兼容格式库：

```toml
[dependencies]
serde_json = "1"
```

如果你直接使用基于 consumer 的扩展示例，需要添加 `qubit-function`：

```toml
[dependencies]
qubit-function = "0.15"
```

本指南中的部分并发示例使用 `qubit-atomic`，从而避免直接暴露标准库 memory ordering 参数：

```toml
[dependencies]
qubit-atomic = "0.13"
```

## 核心模型

一个 progress stream 主要由五个概念组成：

| 概念 | 类型 | 含义 |
| --- | --- | --- |
| Schema | `ProgressSchema` | 一个操作的 metric 字典。 |
| Metric | `ProgressMetric` | 稳定的 metric id 和展示名称。 |
| Counter | `ProgressCounter` | 某个 metric 当前的计数值。 |
| Event | `ProgressEvent` | 一次不可变的进度快照。 |
| Metric snapshot | `ProgressMetricSnapshot` | 一个 event counter 与 metric 元数据和 event 上下文的扁平快照。 |
| Reporter | `ProgressReporter` | 接收 events 的输出端。 |

`Progress` 把这些概念组合成一个逻辑操作。它保存操作开始时间、汇报间隔、可选 stage 和 reporter 引用。

Event 是自描述的：它自带 schema。因此序列化后的 JSON 可以直接被日志、数据库、agent 和外部消费者读取，而不依赖额外的 schema registry。

当 reporter 希望按 metric 粒度处理事件时，可以调用 `ProgressEvent::metric_snapshots()`。每个 `ProgressMetricSnapshot` 都包含完整 `ProgressMetric`、event phase、可选 stage、扁平 counter 值和 elapsed time。

## 快速开始

常见使用流程是：

1. 定义 schema。
2. 选择 reporter。
3. 创建 `Progress` run。
4. 汇报 `started`、`running` 和 terminal event。

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

## 定义 Metric 和 Schema

每一种需要独立统计的单位都应该定义成一个 metric。常见 metric id 包括 `entries`、`files`、`dirs`、`bytes`、`objects`、`requests` 和 `tasks`。

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

如果一个操作只有一个 metric，可以使用 `ProgressSchema::single` 或 `Progress::single_metric`：

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

`ProgressSchema::validate_counter` 和 `ProgressSchema::validate_counters` 是轻量校验。它们检查 counter 的 metric id 是否存在于 schema，并检查同一个 event 中是否重复出现同一个 metric id。它们有意不校验 `completed <= total` 这类数值关系，因为重试、补偿和动态发现总量都是业务语义。

## Counter

`ProgressCounter` 使用 `u64` 保存计数值。这样序列化输出是稳定的，不受当前平台 `usize` 宽度影响。

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

如果操作是开放式的，或者暂时不知道总量，可以使用 `unknown_total` 或不设置 `total`：

```rust
use qubit_progress::ProgressCounter;

let counter = ProgressCounter::new("records").completed(12);
assert_eq!(counter.total_count(), None);
assert_eq!(counter.progress_percent(), None);
```

## 直接构造 Event

如果你只需要 progress 数据模型，不需要 `Progress` 记录 elapsed time 或处理汇报间隔，可以直接构造 event：

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

也可以使用各个生命周期 phase 对应的直接构造函数：

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

## 生命周期 Phase

`ProgressPhase` 有五个值：

| Phase | 含义 |
| --- | --- |
| `Started` | 操作已经开始。 |
| `Running` | 操作仍在执行。 |
| `Finished` | 操作成功完成。 |
| `Failed` | 操作失败。 |
| `Canceled` | 操作被取消。 |

`Finished`、`Failed` 和 `Canceled` 是 terminal phase。

```rust
use qubit_progress::ProgressPhase;

assert_eq!(ProgressPhase::Running.as_str(), "running");
assert!(ProgressPhase::Failed.is_terminal());
```

## Stage

多步骤操作可以使用 `ProgressStage`，例如 scan、copy、verify、publish。Stage 元数据是可选的，可以包含 id、展示名称、序号、总阶段数和相对权重。

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

Stage 可以绑定到整个 `Progress` run，也可以在 event builder 中按 event 覆盖。

## 汇报节奏

需要立即发出 running event 时使用 `report_running`。在高频热路径中需要节流时使用 `report_running_if_due`。

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

当 interval 是 `Duration::ZERO` 时，每次 `report_running_if_due` 都会触发汇报。

## 后台 Running Reporter

当 worker 线程更新共享业务状态，而协调线程需要周期性发出 `running` event 时，可以使用 `Progress::spawn_running_reporter`。

第二个参数是 `snapshot` 闭包：

```rust
FnMut() -> Vec<ProgressCounter>
```

它必须返回下一次 `running` event 要携带的“当前完整 counter 快照”。它返回的不是增量、不是百分比，也不是 `ProgressEvent`。`Progress` 会自动把这些 counters 包装成当前 schema、stage、elapsed time 和 `ProgressPhase::Running`。

返回的 counters 通常应使用创建 `Progress` 时传入的 `ProgressSchema` 中声明过的 metric id。

```rust
// 正确：完整当前状态。
vec![ProgressCounter::new("entries").total(100).completed(42)]

// 不是推荐语义：表示“自上次汇报以来又完成 1 个”。
vec![ProgressCounter::new("entries").completed(1)]
```

后台 reporter 线程会在 running event 可能到期时调用这个 snapshot 闭包。

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
        // 返回 running event 的完整当前快照。
        // 这里表示“总共 3 个，当前完成 1 个”，不是“这次又完成 1 个”。
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

需要注意：

- 必须在 thread scope 退出前调用 `stop_and_join`。
- worker handle 只能汇报 point，不能停止 reporter loop。
- interval 为零时，`RunningProgressPointHandle::report` 会唤醒 reporter loop。
- interval 为正数时，worker point reporting 是低成本 no-op，loop 通过 timeout 自行唤醒。
- reporter 线程中的 panic 会在 `stop_and_join` 中继续传播。

## 内置 Reporter

| Reporter | 适用场景 |
| --- | --- |
| `NoOpProgressReporter` | 测试、可选进度或禁用进度汇报。 |
| `MetricSnapshotProgressReporter` | 把结构化 `ProgressMetricSnapshot` 对象发送给 consumer。 |
| `FormattedProgressReporter` | 格式化每个 metric snapshot，并把字符串发送给 consumer。 |
| `HumanReadableProgressReporter` | 把人类可读 metric snapshot 字符串发送给 consumer。 |
| `JsonProgressReporter` | 把 JSON metric snapshot 字符串发送给 consumer。 |
| `WriterProgressReporter<W>` | 把人类可读 metric snapshot 行写入任意 `Write + Send` sink。 |
| `StdoutProgressReporter` | 命令行 stdout 进度输出。 |
| `StderrProgressReporter` | 命令行 stderr 进度输出。 |
| `LoggerProgressReporter` | 通过 `log` crate 输出进度。 |
| `JsonWriterProgressReporter<W>` | 把 JSON metric snapshot 行写入任意 `Write + Send` sink。 |
| `JsonStdoutProgressReporter` | 命令行 JSON 进度输出到 stdout。 |
| `JsonStderrProgressReporter` | 命令行 JSON 进度输出到 stderr。 |
| `JsonLoggerProgressReporter` | 通过 `log` crate 输出 JSON metric snapshot。 |

使用内存 writer 的示例：

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

## JSON 序列化

`ProgressEvent`、`ProgressSchema`、`ProgressMetric`、`ProgressCounter`、`ProgressPhase` 和 `ProgressStage` 都支持 serde 序列化。`elapsed` 字段使用 `qubit-serde` 的 duration 字符串，例如 `110ms`。

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

这种表示方式适合 agent 读取的日志，因为每个 event 同时包含 metric id 和对应展示名称。

如果需要面向行的结构化输出，优先使用内置 JSON metric snapshot reporter。它们会为每个 metric counter 写出一个 JSON object：

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

已有 `qubit_function::Consumer<String>` 时使用 `JsonProgressReporter`；需要写入任意 `Write` sink 时使用 `JsonWriterProgressReporter`；需要通过 `log` 输出时使用 `JsonLoggerProgressReporter`。

## 直接消费 Metric Snapshot

有些集成不应该把进度格式化成字符串。GUI 进度条、metrics collector 和数据库写入器通常更需要结构化对象。这类场景使用 `MetricSnapshotProgressReporter`：

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

## 实现自定义 Reporter

Reporter 只需要实现一个 trait：

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

`ProgressReporter` 要求 `Send + Sync`，因此 reporter 可以在线程之间共享。如果 reporter 内部需要可变状态，应使用 `Mutex`、`RwLock`、atomic 类型或 channel 这类线程安全机制。

### 用于测试的 Recording Reporter

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

这种方式适合测试，因为 `ProgressEvent` 是不可变且可 clone 的。

### JSON Lines Reporter

JSON-lines reporter 通常适合 agent、批处理任务和外部监控进程。

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

生产代码中应明确 reporter I/O 失败策略：panic、忽略、记录日志，或发送到独立错误 channel。

## GUI 进度条 Reporter

GUI toolkit 通常要求 UI 更新发生在 UI 线程。因此 reporter 不应该在 `report` 中直接操作 widget，而应该发送一条小消息给 UI 线程，由 UI 线程更新进度条。

下面用 `std::sync::mpsc` 表示这个边界：

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

UI 侧可以把消息映射到 widget 状态：

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

真实 GUI crate 中，把 `println!` 替换成 toolkit 相关 UI 更新即可。架构保持一致：reporter 发送消息，UI 线程持有 widget。

## 扩展建议

实现 reporter 时建议：

- 保持 `report` 短小。昂贵工作应交给其他线程或队列。
- 除非进度汇报本身就是操作契约的一部分，否则不要阻塞 worker 热路径。
- 一个 reporter stream 默认对应一个逻辑操作；如果要复用，需要显式增加分流机制。
- 使用 `event.schema()` 解析展示名称，不要在 reporter 中硬编码 metric label。
- 使用 `event.counter("metric_id")` 读取主 metric，使用 `event.counters()` 处理多 metric 展示。
- 明确失败策略：panic、忽略、log，或把错误发送到其他地方。
- 不要在持有锁时调用外部代码。
- GUI 或 async runtime 场景中，优先发送紧凑消息，不要把 UI handle 直接塞进 reporter。

设计 metric id 时建议：

- 使用稳定小写 id，例如 `entries`、`bytes`、`files` 或 `requests`。
- id 面向机器读取；展示文本放在 `ProgressMetric::name`。
- 不同单位使用不同 metric。不要把 bytes 和 files 混在一个 counter 中。
- 同一个操作应保持同一份 schema。

汇报 counter 时建议：

- 已知总量时设置 total。
- stream、发现阶段、开放式任务使用 unknown total。
- 使用 `active` 表示正在执行的单元。
- 需要最终结果统计时使用 `succeeded` 和 `failed`。
- 业务状态保存在 `Progress` 外部；汇报时再构造 fresh counters。

## API 选择建议

| 需求 | 推荐 API |
| --- | --- |
| 构造一个独立 progress payload | `ProgressEvent::builder` |
| 为一个操作记录 elapsed time | `Progress` |
| 在循环中周期性发出 running 更新 | `report_running_if_due` |
| 立即发出 running 更新 | `report_running` |
| worker 线程更新状态，协调线程汇报 | `spawn_running_reporter` |
| 禁用进度汇报 | `NoOpProgressReporter` |
| 人类可读命令行输出 | `StdoutProgressReporter` 或 `StderrProgressReporter` |
| 结构化日志 | 自定义 JSON-lines reporter |
| GUI 进度条 | 自定义 channel-based reporter |

## Crate 边界

`qubit-progress` 有意保持小而清晰。它提供：

- progress 数据模型；
- 操作级生命周期 helper；
- 后台 running report helper；
- reporter trait 和少量内置 reporter。

它不提供：

- terminal UI widget；
- GUI toolkit 集成；
- async runtime 集成；
- 任务调度；
- tracing 基础设施；
- metrics 数据库或 dashboard。

这些集成应由下游 crate 或应用通过实现 `ProgressReporter` 完成。
