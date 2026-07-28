# Qubit Progress

`qubit-progress` 为一次长耗时操作提供简洁、生命周期安全的进度协议。它把不变的操作配置与每次上报时变化的计数彻底分开。

核心规则只有一条：在操作开始时声明指标及其总量；每次上报的闭包只填写当前动态计数。每个 `Event` 都是完整快照，消费者无需依赖之前的事件重建状态。

## 安装

```toml
[dependencies]
qubit-progress = "0.6"
```

使用 JSON Lines 输出时开启 `json-lines` 特性；使用日志 sink 时开启 `log` 特性。

## 快速示例

```rust
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::stderr();
let mut progress = Progress::builder(&reporter)
    .metric(Metric::new("files", "文件").total(2))
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

`Started`、`Running` 和终态事件都会自动携带 `2` 这个总量，业务代码不必反复填写。

## 生命周期与禁用语义

`ProgressBuilder::start` 会校验固定配置、只采样一次 `Reporter::is_enabled()`，并仅在启用时发送 `Started`。禁用后，所有上报和终态闭包均不会执行，不创建事件、不调用 reporter，也不启动后台线程；固定配置校验仍然执行。

生命周期为 `Started → Running* → Succeeded | Failed | Cancelled`。`finish`、`fail` 和 `cancel` 消费 `Progress`，因此安全 Rust 中不能重复发送终态事件，也不能在终态后继续上报。

实现 `Reporter` 即可消费 `&Event`。内置 `NoopReporter`、`TextReporter`、`JsonLinesReporter`（`json-lines` 特性）和 `LogReporter`（`log` 特性）。JSON Lines 每行精确写入一个完整事件，耗时采用 `"250ms"` 这样的规范字符串。

多线程工作可使用 `Progress::spawn_auto_reporter`：它返回有作用域的 `AutoReporter`，独占借用操作；零间隔时工作线程通过可克隆的 `Notifier` 合并唤醒，正间隔时定期心跳。发送终态事件前必须调用 `stop()`。

更多说明见[用户指南](doc/user_guide.zh_CN.md)。
