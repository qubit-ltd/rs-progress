# Qubit Progress 用户指南

`qubit-progress` 表示一次长耗时操作，并把完整事件交给 `Reporter`。README 提供文件复制示例；本指南说明 metric 生命周期、上报调度和工作线程集成。

## 核心模型

- `ProgressBuilder` 在启动前收集 reporter、上报间隔、metrics 和可选的 `Stage`。
- `Metric` 是稳定元数据：机器可读 ID、显示名称和可选总量。
- `Progress` 持有每个 metric 的动态状态。通过 `Progress::metric` 取得可克隆的 `MetricHandle` 后，调用方只需执行状态转换，无需维护外部计数器。
- `Event` 是不可变的完整观察结果；reporter 无需依赖先前事件重建状态。

生命周期为 `Started → Running* → Succeeded | Failed | Cancelled`。`finish`、`fail` 和 `cancel` 会消费 `Progress`，因此安全 Rust 中最多只能发送一个终态事件。终态前丢弃对象或 unwind 仍可能使操作没有终态事件。

## 启动并更新 metric

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

`start()` 在发送 `Started` 前校验固定元数据，`report()` 将当前 metric 快照作为 `Running` 发送。终态方法返回耗时 `Duration`；终态投递失败时，`TerminalError` 同时保留耗时和底层 `ProgressError`。

## Metric 生命周期与校验

`start(count)` 的正数把数量从未开始移动到 active，负数执行反向移动。`complete`、`succeed`、`fail` 和 `cancel` 的正数把 active 移动到对应完成状态，负数执行反向移动。任何计数都不能变为负数；已知总量时，`active + completed` 不能超过总量，`set_total` 也不能把总量下调到低于该已占用数量。

`completed` 包含未分类完成、成功、失败和取消。每次转换均在内部锁中校验后提交，因此每个发送出的 metric 快照都内部一致。不要用计数表达阶段信息：在启动时使用 `Stage`，用 `set_stage` 替换后续事件的阶段，或用 `clear_stage` 清除它。

## 选择上报调度

`report()` 立即尝试发送 `Running`，`report_if_due()` 则遵守 builder 的间隔：

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
    // 处理一条记录。
    records.start(1)?;
    records.succeed(1)?;
    progress.report_if_due()?;
}
progress.finish()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

间隔为零时，每次 `report_if_due()` 都到期。reporter 失败会消耗一个投递序号并重置下次截止时间。

## 自动上报工作线程的状态

`spawn_auto_reporter` 会上报由工作线程更新的 metric 状态。它启动一个有作用域的后台线程并独占可变的 `Progress` 借用；返回的 `AutoReporter` 提供可克隆的 `Notifier` 和 `Status`。

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
    let notifier = auto.notifier();
    let worker = scope.spawn(move || {
        files.start(3).expect("metric update must succeed");
        files.succeed(3).expect("metric update must succeed");
        notifier.notify();
    });
    worker.join().expect("copy worker panicked");
    auto.stop()?;
    Ok(())
})?;

progress.finish()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

零间隔下，`notify()` 会把重复通知合并成最多一个待发送上报。正间隔下后台线程按最小间隔发送心跳，`notify()` 是无操作。`notify()` 在 `stop()` 后无害；调用终态方法前必须调用 `stop()` 并处理其结果。

## 禁用操作与 reporters

`Reporter::is_enabled()` 的结果由 `start()` 采样一次。禁用操作仍会校验固定配置并维护 metric 状态，但不发送事件，也不启动自动上报线程。`NoopReporter` 可显式禁用输出。

内置 reporters 包括 `TextReporter<W>`、`NoopReporter`、启用 `json-lines` feature 后的 `JsonLinesReporter<W>`，以及启用 `log` feature 后的 `LogReporter`。自定义 reporter 实现 `Reporter` 并消费 `&Event`，且必须是 `Send + Sync`。
