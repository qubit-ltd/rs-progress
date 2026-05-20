# Qubit Progress

[![Rust CI](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-progress/coverage-badge.json)](https://qubit-ltd.github.io/rs-progress/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-progress.svg?color=blue)](https://crates.io/crates/qubit-progress)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

面向 Qubit Rust 库和应用的通用进度汇报抽象。

## 概述

Qubit Progress 把进度建模为不可变、自描述的事件。每个事件都携带 metric schema、生命周期 phase、可选 stage 信息、metric counters 和 elapsed time。Reporter 接收完整事件快照，可以把它渲染为人类可读文本、日志、JSON 或应用自己的记录格式。

适合使用本 crate 的场景包括：

- 需要为一个操作定义稳定的 metric，例如 files、bytes、entries 或 tasks；
- 需要按 metric id 分组的 `u64` counter；
- 需要 `started`、`running`、`finished`、`failed`、`canceled` 等生命周期 phase；
- 需要为多阶段操作附加可选 stage 元数据；
- 需要以一个逻辑操作为作用域，并按可配置间隔汇报 running 进度；
- 需要为 worker 驱动的操作提供后台 running reporter；
- 需要支持 serde 序列化、便于日志、agent 和结构化消费者读取的 progress event。

详细用法、扩展示例和 reporter 设计建议请参见[中文用户指南](doc/user_guide.zh_CN.md)。
API 参考文档可在 [docs.rs](https://docs.rs/qubit-progress) 查看。

## 安装

```toml
[dependencies]
qubit-progress = "0.5"
```

## 快速示例

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

## 主要能力

### Schema 与 Metric

`ProgressSchema` 定义一个 progress event stream 中可能出现的 metric 维度。一个 metric 包含用于结构化数据的稳定 `id`，以及用于展示的人类可读 `name`。

| 类型 | 用途 |
| --- | --- |
| `ProgressSchema` | 一个逻辑操作的 metric 定义 |
| `ProgressMetric` | 稳定 metric id 和展示名称 |
| `ProgressCounter` | 某个 metric id 对应的 `u64` 计数 |
| `ProgressStage` | 可选的多阶段操作元数据 |

一个 schema 可以包含多个 metric，例如 `entries` 和 `bytes`。这样单个 event 就能同时汇报逻辑条目进度和字节进度，而不会混淆单位。

### Event 与 Counter

`ProgressEvent` 是不可变快照，包含：

| 字段 | 用途 |
| --- | --- |
| `schema` | event 自带的 metric 定义 |
| `phase` | 生命周期状态：`started`、`running`、`finished`、`failed` 或 `canceled` |
| `stage` | 可选的多阶段操作元数据 |
| `counters` | 一个或多个按 `metric_id` 分组的 `ProgressCounter` |
| `elapsed` | elapsed `Duration`，通过 `qubit-serde` 序列化为 `110ms` 这类字符串 |

可以直接使用 `ProgressEvent::builder(schema)` 构造事件：

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressEvent,
    ProgressMetric,
    ProgressSchema,
};

let event = ProgressEvent::builder(ProgressSchema::single("entries", "Entries"))
    .running()
    .counter("entries", |counter| counter.total(5).completed(2))
    .elapsed(Duration::from_millis(110))
    .build();

assert_eq!(event.counter("entries").map(|counter| counter.completed_count()), Some(2));
```

### 操作级 Progress

一个 `Progress` 实例只对应一个逻辑操作。不要把多个无关操作混进同一个 reporter 事件流，除非该 reporter 明确支持复用和分流。多线程任务应先聚合 counters，再通过这个操作级 `Progress` 统一汇报。

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

### 后台汇报线程

当 worker 线程更新业务状态，而协调线程需要周期性发出 `running` 事件时，可以使用 `Progress::spawn_running_reporter`。worker 更新共享状态后调用 `RunningProgressPointHandle::report()`；当间隔为 `Duration::ZERO` 时，这个调用会唤醒后台汇报线程。
`RunningProgressGuard` 持有这个后台汇报线程，`RunningProgressPointHandle` 是可克隆的 worker 侧唤醒句柄。

下面的示例使用 `qubit-atomic` 的 `ArcAtomic`；如果复制这个模式到自己的 crate，需要额外添加 `qubit-atomic = "0.13"`。

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
        vec![ProgressCounter::new("entries")
            .total(3)
            .completed(snapshot_completed.load())]
    });
    let point = running.point_handle();

    completed.store(1);
    assert!(point.report());

    running.stop_and_join();
});
```

### Reporter 实现

Reporter 通过 `ProgressReporter` 接收不可变的 `ProgressEvent`：

```rust
fn report(&self, event: &ProgressEvent);
```

内置 reporter：

| Reporter | 用途 |
| --- | --- |
| `NoOpProgressReporter` | 忽略事件 |
| `WriterProgressReporter` | 把人类可读文本写入任意 `Write` sink |
| `StdoutProgressReporter` | 写入 stdout |
| `StderrProgressReporter` | 写入 stderr |
| `LoggerProgressReporter` | 通过 `log` crate 输出 |

Reporter 可以按 `metric_id` 分组，并通过 `event.schema()` 解析适合展示的 metric 名称。

## JSON 序列化

Progress event 支持 serde 序列化。`elapsed` 使用 `qubit-serde` 的 `duration_with_unit` 适配器，因此 JSON 更紧凑，也更适合 agent 读取。

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

## Crate 边界

`qubit-progress` 提供 progress 数据模型、操作级生命周期 helper 和 reporter 抽象。它有意不提供终端 UI 控件、async runtime 集成、任务调度、tracing 基础设施或长期 metrics 存储。

## 运行时依赖

本 crate 依赖：

- `serde`：用于可序列化的 progress model；
- `log`：用于 `LoggerProgressReporter`；
- `qubit-serde`：用于紧凑的 `Duration` 序列化。

它不要求 async runtime。

## 测试与代码覆盖率

本项目为 progress model、event builder、汇报间隔、后台汇报、reporter 实现和 JSON 序列化保持测试覆盖。

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行覆盖率报告
./coverage.sh

# 生成文本格式报告
./coverage.sh text

# 运行 CI 检查（格式化、clippy、测试、覆盖率、audit）
./ci-check.sh
```

## 许可证

Copyright (c) 2026. Haixing Hu.

根据 Apache 许可证 2.0 版（"许可证"）授权；
除非遵守许可证，否则您不得使用此文件。
您可以在以下位置获取许可证副本：

    http://www.apache.org/licenses/LICENSE-2.0

除非适用法律要求或书面同意，否则根据许可证分发的软件
按"原样"分发，不附带任何明示或暗示的担保或条件。
有关许可证下的特定语言管理权限和限制，请参阅许可证。

完整的许可证文本请参阅 [LICENSE](LICENSE)。

## 贡献

欢迎贡献。请随时提交 Pull Request。

### 开发指南

- 遵循 Rust API 指南。
- 将 progress reporting 相关能力保留在 `qubit-progress` 中。
- 保持 event 不可变和自描述。
- 保持 reporter 实现小而明确。
- 保持全面的测试覆盖。
- 公共 API 在有助于说明行为时应提供文档和示例。
- 提交 PR 前确保 `./ci-check.sh` 通过。

## 作者

**Haixing Hu**

## 相关项目

- [qubit-serde](https://github.com/qubit-ltd/rs-serde)：Qubit Rust crate 使用的 serde helper。
- Qubit 旗下的更多 Rust 库发布在 GitHub 组织 [qubit-ltd](https://github.com/qubit-ltd)。

---

仓库地址：[https://github.com/qubit-ltd/rs-progress](https://github.com/qubit-ltd/rs-progress)
