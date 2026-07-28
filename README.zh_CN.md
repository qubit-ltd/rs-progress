# Qubit Progress

[![Rust CI](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-progress/coverage-badge.json)](https://qubit-ltd.github.io/rs-progress/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-progress.svg?color=blue)](https://crates.io/crates/qubit-progress)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

面向 Qubit Rust 库和应用的通用进度汇报抽象。

## 概述

长时间运行的库代码和命令行应用，通常需要把进度汇报给 stderr、日志、JSON 流或
GUI。如果在业务循环中直接打印，就会把操作和某一种展示方式耦合；如果每个操作都
自行定义回调参数，消费者也无法稳定理解 metric id、生命周期、stage 和 elapsed
time。

Qubit Progress 把生产端和消费端分开。业务代码拥有工作状态，把状态转换为
counter，并通过 `Progress` 发出不可变、自描述的 `ProgressEvent` 快照；
`ProgressReporter` 消费每个 event，决定如何展示或存储它。

```text
业务状态 → ProgressCounter 快照 → ProgressEvent → ProgressReporter → stderr / log / JSON / GUI
```

每个 event 都携带 metric schema、生命周期 phase、可选 stage、metric counter 和
elapsed time。因此同一个操作可以使用统一的进度协议，而调用方仍能选择自己的输出
sink。

适合使用本 crate 的场景包括：

- 需要为一个操作定义稳定的 metric，例如 files、bytes、entries 或 tasks；
- 需要按 metric id 分组的 `u64` counter；
- 需要 `started`、`running`、`finished`、`failed`、`canceled` 等生命周期 phase；
- 需要为多阶段操作附加可选 stage 元数据；
- 需要以一个逻辑操作为作用域，并按可配置间隔汇报 running 进度；
- 需要为 worker 驱动的操作提供后台 running reporter；
- 需要支持 serde 序列化、便于日志、agent 和结构化消费者读取的 progress event。

完整的文件复制流程、错误路径说明和扩展示例请参见[中文用户指南](doc/user_guide.zh_CN.md)。
API 参考文档可在 [docs.rs](https://docs.rs/qubit-progress) 查看。

## 安装

```toml
[dependencies]
qubit-progress = "0.6"
```

## 快速示例

下面的命令复制一批文件。复制循环负责产生进度；`StderrProgressReporter` 是消费端，
负责把每个已交付 event 渲染给执行命令的人。

```rust
use std::{
    error::Error,
    time::Duration,
};

use qubit_progress::{
    Progress,
    ProgressMetric,
    ProgressSchema,
    StderrProgressReporter,
};

fn main() -> Result<(), Box<dyn Error>> {
    let files = [
        ("input/january.csv", "backup/january.csv"),
        ("input/february.csv", "backup/february.csv"),
    ];
    let total_files = u64::try_from(files.len())?;
    let total_bytes = files.iter().try_fold(0_u64, |total, (source, _)| {
        Ok::<_, std::io::Error>(total + std::fs::metadata(*source)?.len())
    })?;

    let schema = ProgressSchema::new(vec![
        ProgressMetric::new("files", "Files"),
        ProgressMetric::new("bytes", "Bytes"),
    ]);
    let reporter = StderrProgressReporter::new();
    let mut progress = Progress::new(&reporter, Duration::from_millis(500), schema);

    progress.report_started(|event| {
        event
            .counter("files", |counter| counter.total(total_files))
            .counter("bytes", |counter| counter.total(total_bytes))
    })?;

    std::fs::create_dir_all("backup")?;
    let mut completed_files = 0_u64;
    let mut completed_bytes = 0_u64;
    for (source, destination) in files {
        completed_bytes += std::fs::copy(source, destination)?;
        completed_files += 1;
        progress.report_running_if_due(|event| {
            event
                .counter("files", |counter| counter.total(total_files).completed(completed_files))
                .counter("bytes", |counter| counter.total(total_bytes).completed(completed_bytes))
        })?;
    }

    progress.report_finished(|event| {
        event
            .counter("files", |counter| counter.total(total_files).completed(total_files).succeeded(total_files))
            .counter("bytes", |counter| counter.total(total_bytes).completed(total_bytes).succeeded(total_bytes))
    })?;
    Ok(())
}
```

`report_started`、每个到期的 `report_running_if_due` 和 `report_finished` 都会构造
`ProgressEvent`，并同步调用 `ProgressReporter::report`。在这个命令行场景中，
`StderrProgressReporter` 通过把每个 metric 渲染为一行人类可读文本并写入 stderr 来
消费 event。替换 reporter 即可改为 JSON、日志或结构化 snapshot，不需要改动复制
循环。

该示例聚焦成功路径。真实命令在 `std::fs::copy` 失败时，应先用最新 counter 调用
`report_failed`，再返回复制错误；reporter 输出失败也应向上传播，不能静默丢失进度。

完整场景、reporter 选择和自定义消费者实现请参见[中文用户指南](doc/user_guide.zh_CN.md)。

## 主要能力

### Schema 与 Metric

`ProgressSchema` 定义一个 progress event stream 中可能出现的 metric 维度。一个 metric 包含用于结构化数据的稳定 `id`，以及用于展示的人类可读 `name`。

| 类型 | 用途 |
| --- | --- |
| `ProgressSchema` | 一个逻辑操作的 metric 定义 |
| `ProgressMetric` | 稳定 metric id 和展示名称 |
| `ProgressCounter` | 某个 metric id 对应的 `u64` 计数 |
| `ProgressMetricSnapshot` | 一个 metric counter 与 event phase、stage、elapsed time 的扁平快照 |
| `ProgressStage` | 可选的多阶段操作元数据 |

一个 schema 可以包含多个 metric，例如 `entries` 和 `bytes`。这样单个 event 就能同时汇报逻辑条目进度和字节进度，而不会混淆单位。

### Event 与 Counter

`ProgressEvent` 是不可变快照，包含：

| 字段 | 用途 |
| --- | --- |
| `schema` | event 自带的 metric 定义 |
| `operation_id` | 一个逻辑操作共享的进程内标识 |
| `phase` | 生命周期状态：`started`、`running`、`finished`、`failed` 或 `canceled` |
| `stage` | 可选的多阶段操作元数据 |
| `counters` | 一个或多个按 `metric_id` 分组的 `ProgressCounter` |
| `elapsed` | elapsed `Duration`，通过 `qubit-datatype` 序列化为 `110ms` 这类字符串 |

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

progress
    .report_started(|event| event.counter("entries", |counter| counter.total(3)))
    .expect("progress output should succeed");

progress.report_running(|event| {
    event
        .counter("entries", |counter| counter.total(3).completed(1).active(1))
        .counter("bytes", |counter| counter.total(1_024).completed(512))
}).expect("progress output should succeed");

progress.report_finished(|event| {
    event
        .counter("entries", |counter| counter.total(3).completed(3).succeeded(3))
        .counter("bytes", |counter| counter.total(1_024).completed(1_024))
}).expect("progress output should succeed");
```

`report_running_if_due` 只有在达到汇报间隔时才会调用 builder 闭包，因此正数间隔下的热路径开销很低。

### 后台汇报线程

当 worker 线程更新业务状态，而协调线程需要周期性发出 `running` 事件时，可以使用 `Progress::spawn_running_reporter`。worker 更新共享状态后调用 `RunningProgressPointHandle::report()`；当间隔为 `Duration::ZERO` 时，这个调用会唤醒后台汇报线程。
`RunningProgressGuard` 持有这个后台汇报线程，`RunningProgressPointHandle` 是可克隆的 worker 侧唤醒句柄。
可通过 `RunningProgressGuard::status` 在 join 前观察后台输出失败；
返回的 `RunningProgressStatus` 可克隆；`stop_and_join` 仍会返回其错误或继续传播 panic。

下面的示例使用 `qubit-atomic` 的 `ArcAtomic`；如果复制这个模式到自己的 crate，需要额外添加 `qubit-atomic = "0.13"`。

```rust
use std::{
    thread,
    time::Duration,
};

use qubit_atomic::ArcAtomic;
use qubit_progress::{
    Progress,
    ProgressCounter,
    ProgressSchema,
    WriterProgressReporter,
};

let reporter = WriterProgressReporter::from_writer(Vec::new());
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
    let status = running.status();

    completed.store(1);
    point.report();

    assert!(!status.is_failed());
    running.stop_and_join().expect("progress output should succeed");
});
```

### Reporter 实现

Reporter 通过 `ProgressReporter` 接收不可变的 `ProgressEvent`：

```rust
fn report(&self, event: &ProgressEvent) -> Result<(), ProgressReportError>;
```

内置 reporter：

| Reporter | 用途 |
| --- | --- |
| `NoOpProgressReporter` | 忽略事件 |
| `MetricSnapshotProgressReporter` | 把结构化 `ProgressMetricSnapshot` 对象发送给 consumer |
| `FormattedProgressReporter` | 格式化每个 metric snapshot，并把字符串发送给 consumer |
| `HumanReadableProgressReporter` | 把人类可读 metric snapshot 字符串发送给 consumer |
| `JsonProgressReporter` | 把 JSON metric snapshot 字符串发送给 consumer |
| `WriterProgressReporter` | 把人类可读 metric snapshot 行写入任意 `Write` sink |
| `StdoutProgressReporter` | 写入 stdout |
| `StderrProgressReporter` | 写入 stderr |
| `LoggerProgressReporter` | 通过 `log` crate 输出 |
| `JsonWriterProgressReporter` | 把 JSON metric snapshot 行写入任意 `Write` sink |
| `JsonStdoutProgressReporter` | 把 JSON metric snapshot 写入 stdout |
| `JsonStderrProgressReporter` | 把 JSON metric snapshot 写入 stderr |
| `JsonLoggerProgressReporter` | 通过 `log` crate 输出 JSON metric snapshot |

Reporter 可以调用 `event.metric_snapshots_iter()`，惰性地把每个 counter 转换成包含完整 metric 对象、phase、可选 stage、扁平 counter 值和 elapsed time 的 `ProgressMetricSnapshot`。

## JSON 序列化

启用 `serde` feature 后，progress event 支持 serde 序列化。`elapsed` 使用
`qubit-datatype` 的 `duration_with_unit` 适配器，因此 JSON 更紧凑，也更适合
agent 读取。
该适配器会自动选择能够精确表示数值的最大单位，因此亚毫秒 elapsed 也能无损
round-trip。反序列化要求规范的 duration 文本，不会 trim 两侧空白。

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
let value: serde_json::Value = serde_json::from_str(&json)
    .expect("serialized event should be valid JSON");
assert!(value["operation_id"].as_u64().is_some_and(|id| id > 0));
assert_eq!(value["elapsed"], "110ms");
```

## Crate 边界

`qubit-progress` 提供 progress 数据模型、操作级生命周期 helper 和 reporter 抽象。它有意不提供终端 UI 控件、async runtime 集成、任务调度、tracing 基础设施或长期 metrics 存储。

## 运行时依赖

`serde` feature 提供可序列化的 progress model，并通过 `qubit-datatype`
提供紧凑的 `Duration` 序列化；默认启用。`json` feature 会隐含启用 `serde`，
默认 feature 还会启用 `consumer-reporters` 与 `log`。如果只需要无依赖的
核心 model、生命周期 API、no-op reporter 与 writer reporter，可以关闭默认
feature：

```toml
qubit-progress = { version = "0.6", default-features = false }
```

本 crate 不要求 async runtime。

## 测试

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./ci-check.sh

# 检查代码覆盖率
./coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./align-ci.sh`格式化代码，运行`./ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-progress](https://github.com/qubit-ltd/rs-progress)
