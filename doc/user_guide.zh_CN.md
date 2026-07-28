# Qubit Progress 用户指南

`qubit-progress` 表示一次长耗时操作，并向 `Reporter` 投递完整事件。文件复制的快速上手请见 [README](../README.zh_CN.md)；需要选择指标、调度上报、集成工作线程或输出 sink 时，请查阅本指南。

## 核心模型

一次操作包含稳定配置与变化状态两部分：

- `ProgressBuilder` 在操作开始前收集 reporter、上报间隔、指标和可选 `Stage`。
- `Metric` 是稳定元数据：机器可读 ID、显示名称和可选总量。每个事件都会携带它。
- `Snapshot` 仅在一次上报闭包中存在，用于填写当前的 `completed`、`active`、`succeeded` 与 `failed` 计数。
- `Event` 是不可变的完整观测值；reporter 无需依赖之前的事件重建状态。

生命周期为 `Started → Running* → Succeeded | Failed | Cancelled`。`finish`、`fail` 和 `cancel` 会消费 `Progress`，因此安全 Rust 中无法再发送第二个终态事件，也不能在终态后继续上报。

## 启动操作并上报快照

代码中使用稳定指标 ID，输出中使用可读名称；已知总量时只需配置一次。下面的操作先上报一个批次的进行中状态，再结束操作。

```rust
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(std::io::stderr());
let mut progress = Progress::builder(&reporter)
    .metric(Metric::new("files", "文件").total(10))
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

`start()` 会先校验全部稳定元数据，之后才发送 `Started`；`report()` 会先校验新快照，之后才发送 `Running`。终态方法返回耗时 `Duration`；若投递终态失败，`TerminalError` 同时保留该耗时和底层 `ProgressError`。

## 快照规则与校验

每个快照必须恰好配置一次每个已声明的指标。空白或重复指标 ID、在 `Snapshot::metric` 中使用未知 ID、重复更新、互相矛盾的计数，以及超过已配置总量的值，都会导致校验错误。尤其是 `succeeded + failed` 不得大于 `completed`；存在总量时，`completed + active` 不得超过总量。

固定元数据应通过 `Metric` 配置，动态数值应在上报闭包中填写。不要把阶段信息藏在计数器中：启动时用 `Stage`，后续事件要替换阶段则调用 `set_stage`，要移除阶段则调用 `clear_stage`。只有在操作开始后才得知或需要改变总量时，才调用 `set_total`。

## 选择上报调度方式

`report()` 会立即尝试发送 `Running`。`report_if_due()` 则遵守 builder 设置的间隔：

```rust
use std::time::Duration;
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(std::io::stderr());
let mut progress = Progress::builder(&reporter)
    .interval(Duration::from_secs(1))
    .metric(Metric::new("records", "记录"))
    .start()?;

for completed in 1..=100 {
    // 处理一条记录。
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

间隔为零表示每次 `report_if_due()` 都到期。操作尚未到期时，闭包不会执行。reporter 失败会消耗一个投递序号并重置下次截止时间；快照校验失败则两者都不会发生。

## 自动上报工作线程更新的状态

`spawn_auto_reporter` 用于 `Progress` 外部保存的状态，例如文件复制工作线程共享的计数器。它会启动一个有作用域的后台线程，独占可变的 progress 借用并调用快照闭包。返回的 `AutoReporter` 提供可克隆的 `Notifier` 与 `Status`。

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
    .metric(Metric::new("files", "文件").total(3))
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

间隔为零时，`notify()` 会把多次调用合并为至多一次待处理上报。正间隔时，后台线程按最小间隔发送心跳；通知只会唤醒等待，不会绕过限速。`stop()` 后调用 `notify()` 没有副作用。必须在终态方法前调用 `stop()` 并处理其结果。`AutoReporter` 存在期间，独占借用会阻止手工上报、修改阶段、修改总量和结束操作。

## 禁用的操作

启用状态由 `Reporter::is_enabled()` 决定，并由 `start()` 只采样一次。禁用操作仍会校验固定配置，但不发送 `Started`；上报和终态闭包都不会执行；不会分配事件；自动上报也不会启动后台线程。因此，当 sink 被禁用时，调用方仍可无条件采用同一条上报路径且成本很低。

当调用方需要显式禁用输出的 reporter 时，可使用 `NoopReporter`。自定义 reporter 可覆写 `is_enabled()`，把启用逻辑接入应用配置。

## Reporter 与结构化输出

实现 `Reporter` 即可消费 `&Event`；实现必须满足 `Send + Sync`。内置 reporter 包括：

- `TextReporter<W>`：向任意 `Write + Send` 目标逐事件写入人类可读的一行文本。
- `NoopReporter`：禁用事件。
- `JsonLinesReporter<W>`：开启 `json-lines` 后可用，每行写入一个完整 JSON 事件。
- `LogReporter`：开启 `log` 后可用，转发事件到 `log` 生态。

JSON Lines 适合日志采集与后处理，因为每一行都是完整事件。耗时会编码为整数加上 `h`、`m`、`s`、`ms`、`us`、`ns` 中最大的精确单位；反序列化时会执行与运行时构造相同的公开不变量校验。

## 错误与关闭

多数非终态操作返回 `Result<(), ProgressError>`。`ProgressError` 表示无效进度数据或 reporter 返回的失败，不能忽略：无效快照表示事件没有投递，sink 错误表示该次投递尝试失败。

终态方法返回 `Result<Duration, TerminalError>`。当必须可靠记录完成状态时，应检查 `TerminalError`：即使终态事件无法投递，它仍会保留耗时。自动上报场景中，`AutoReporter::stop()` 返回后台线程的校验或 reporter 错误；若工作线程 panic，则会在调用线程恢复 panic。

## 延伸参考

每个类型、错误变体、reporter 特性与序列化细节，请查阅 [docs.rs](https://docs.rs/qubit-progress) 生成的 API 文档。
