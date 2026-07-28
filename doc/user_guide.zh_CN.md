# Qubit Progress 用户指南

## 模型

一个 `Progress` 对应一次操作。`Metric` 是固定配置：稳定 ID、显示名称和可选总量。`Snapshot` 只在一次上报闭包内短暂存在，只能填写 `completed`、`active`、`succeeded`、`failed` 等动态计数。

```rust
use qubit_progress::{Metric, NoopReporter, Progress};

let reporter = NoopReporter;
let mut progress = Progress::builder(&reporter)
    .metric(Metric::new("tasks", "任务").total(10))
    .start()?;

progress.report(|snapshot| {
    snapshot.metric("tasks", |counts| {
        counts.completed(4).succeeded(3).failed(1).active(2);
    });
})?;

progress.finish(|snapshot| {
    snapshot.metric("tasks", |counts| {
        counts.completed(10).succeeded(9).failed(1);
    });
})?;
# Ok::<(), qubit_progress::TerminalError>(())
```

校验会拒绝重复或空白的指标 ID、矛盾计数以及超过已知总量的计数。阶段信息使用 `Stage` 明确配置或更新，不混在计数中。

## 调度与终态

`report` 总是尝试发送 `Running`；`report_if_due` 会遵守配置的最小间隔，未到时间时连闭包也不会执行。间隔为零时每次调用都到期。reporter 失败会消耗一次投递序号，快照校验失败不会。终态方法消费操作并返回耗时；终态投递失败时，`TerminalError` 同时保留耗时和底层 `ProgressError`。

## 禁用语义

启用状态属于 reporter，并在 `start` 时只采样一次。禁用后，上报闭包、终态闭包均不会执行，不创建事件、不调用 reporter，也不会为自动上报创建线程。因此调用方可以始终无条件写同一条上报路径，而无需任何 `*_if_enabled` API。

## 自动上报

`spawn_auto_reporter` 返回有作用域的 `AutoReporter`，其独占借用 `Progress`；在它存活期间不能手工上报或终态。零间隔时工作线程通过 `Notifier::notify` 合并唤醒；正间隔时按最小间隔发送心跳，通知只会唤醒等待而不会绕过间隔。`stop` 后通知自动成为 no-op，随后才可发送终态事件。

## 结构化输出

开启 `json-lines` 特性后，`JsonLinesReporter` 每行写入一个完整 `Event`。耗时字段采用整数加 `h`、`m`、`s`、`ms`、`us` 或 `ns`，序列化时使用最大的精确单位；反序列化会执行与运行时构造相同的公开不变量校验。
