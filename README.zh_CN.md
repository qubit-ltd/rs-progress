# Qubit Progress

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-progress.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-progress)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-progress/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-progress?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-progress.svg?color=blue)](https://crates.io/crates/qubit-progress)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English](https://img.shields.io/badge/docs-English-blue.svg)](README.md)

Qubit Rust 库使用的通用进度汇报抽象。

## 什么时候使用

当一个操作需要汇报进度，但又不希望进度 API 绑定到某个具体业务领域时，可以使用
`qubit-progress`：

- 安装程序需要依次汇报准备、复制、校验、清理等阶段；
- 批处理任务需要统一的计数器和耗时信息；
- 命令行工具希望在控制台输出和日志输出之间切换；
- 库代码希望暴露进度快照，但不依赖具体运行时。

这个 crate 不是调度器、任务执行器或 UI 框架。它只定义进度事件模型和基础 reporter。

## 概览

Qubit Progress 将进度建模为不可变事件。一个进度事件包含：

- `ProgressPhase`：生命周期阶段，例如 started、running、finished、failed、
  canceled。
- `ProgressStage`：多阶段操作的可选阶段信息。
- `ProgressCounters`：通用的 total、completed、active、succeeded、failed
  计数器。
- `std::time::Duration` 表示的已耗时。
- 调用方定义的上下文，用于携带业务细节。
- `ProgressReporter<C>`：接收带类型上下文的进度事件。
- `NoOpProgressReporter`、`WriterProgressReporter` 和
  `LoggerProgressReporter`：可复用的 reporter 实现。

业务 crate 应保留自己的领域状态，并在汇报进度时转换成 `ProgressEvent<C>` 快照。
例如 `qubit-batch` 可以把批处理进度快照作为事件上下文，同时复用通用的生命周期、
阶段和 reporter 机制。

## 安装

```toml
[dependencies]
qubit-progress = "0.1"
```

## 快速开始

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressCounters,
    ProgressEvent,
    ProgressReporter,
    ProgressStage,
    WriterProgressReporter,
};

let reporter = WriterProgressReporter::from_writer(Vec::<u8>::new());
let counters = ProgressCounters::new(Some(4))
    .with_completed_count(2)
    .with_active_count(1);
let event = ProgressEvent::running(counters, Duration::from_secs(2), ())
    .with_stage(ProgressStage::new("copy", "Copy files"));

reporter.report(&event);
```

## 阶段化进度

`ProgressStage` 描述操作处于哪个业务阶段。它和生命周期阶段不同：复制文件这个
stage 可以处于 running、finished、failed 或 canceled 等 phase。

```rust
use std::time::Duration;

use qubit_progress::{
    ProgressCounters,
    ProgressEvent,
    ProgressPhase,
    ProgressStage,
};

let stage = ProgressStage::new("verify", "Verify installation")
    .with_index(2)
    .with_total_stages(5)
    .with_weight(0.2);

let event = ProgressEvent::new(
    ProgressPhase::Running,
    ProgressCounters::new(Some(10)).with_completed_count(7),
    Duration::from_secs(12),
    "checking files",
)
.with_stage(stage);

assert_eq!(event.phase(), ProgressPhase::Running);
assert_eq!(event.context(), &"checking files");
```

## 计数语义

`ProgressCounters` 支持已知总量和未知总量两类进度：

- `total_count: Some(n)` 表示可以计算百分比和剩余数量。
- `total_count: None` 表示开放式操作，或暂时不知道总量。
- `completed_count` 表示已经到达终态的工作单元数量。
- `active_count` 表示当前正在执行的工作单元数量。
- `succeeded_count` 和 `failed_count` 是可选的聚合结果计数，供能汇报这些
  信息的业务使用。

当总量已知且为零时，进度被视为完成：

```rust
use qubit_progress::ProgressCounters;

let counters = ProgressCounters::new(Some(0));
assert_eq!(counters.progress_percent(), Some(100.0));
```

## Reporter 行为

Reporter 本质上会产生副作用。它可以写终端、写文件、发日志、更新 UI 桥接层，
也可以在测试中记录事件。如果 reporter panic，调用方自行决定传播还是隔离。

`WriterProgressReporter` 会写出紧凑的人类可读文本。它不读取事件上下文，因此可以
汇报任意 `ProgressEvent<C>`。

`LoggerProgressReporter` 通过 `log` crate 输出，并支持配置 target 和 level。

## 公共 API 速查

- `ProgressPhase`：生命周期阶段枚举。
- `ProgressStage`：阶段 id、名称、索引、总阶段数和可选权重。
- `ProgressCounters`：通用计数器，提供剩余数量和百分比辅助方法。
- `ProgressEvent<C>`：不可变事件，携带 phase、stage、counters、elapsed 和上下文。
- `ProgressReporter<C>`：接收进度事件的 trait。
- `NoOpProgressReporter`：忽略所有事件的 reporter。
- `WriterProgressReporter<W>`：基于 writer 的人类可读 reporter。
- `LoggerProgressReporter`：基于 `log` crate 的 reporter。

## 项目结构

- `src/progress_phase.rs`：生命周期阶段枚举。
- `src/progress_stage.rs`：多阶段元数据。
- `src/progress_counters.rs`：通用计数器模型。
- `src/progress_event.rs`：不可变事件类型。
- `src/progress_reporter.rs`：reporter trait。
- `src/writer_progress_reporter.rs`：基于 writer 的 reporter。
- `src/logger_progress_reporter.rs`：基于 logger 的 reporter。
- `tests/progress`：计数器、事件和 reporter 的行为测试。

## 文档

- API 文档：[docs.rs/qubit-progress](https://docs.rs/qubit-progress)
- Crate 发布页：[crates.io/crates/qubit-progress](https://crates.io/crates/qubit-progress)
- 源码仓库：[github.com/qubit-ltd/rs-progress](https://github.com/qubit-ltd/rs-progress)

## 测试和 CI

在 crate 根目录运行快速本地检查：

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

要尽量匹配仓库 CI 环境，运行：

```bash
./align-ci.sh
./ci-check.sh
./coverage.sh json
```

`./align-ci.sh` 会同步本地工具链和 CI 相关配置；`./ci-check.sh` 运行与流水线一致的
检查。修改需要覆盖率体现的行为时，可以运行 `./coverage.sh`。

## 贡献

欢迎 issue 和 pull request。请保持改动聚焦；行为变化时补充或更新测试；公共 API 或
用户可见行为变化时同步更新 README 或 rustdoc。

贡献代码即表示你同意贡献内容使用同一份 [Apache License, Version 2.0](LICENSE) 许可。

## 许可和版权

Copyright © 2026 Haixing Hu, Qubit Co. Ltd.

本软件使用 [Apache License, Version 2.0](LICENSE) 许可。

## 作者和维护

**胡海星** — Qubit Co. Ltd.

| | |
| --- | --- |
| **源码仓库** | [github.com/qubit-ltd/rs-progress](https://github.com/qubit-ltd/rs-progress) |
| **API 文档** | [docs.rs/qubit-progress](https://docs.rs/qubit-progress) |
| **Crate 发布** | [crates.io/crates/qubit-progress](https://crates.io/crates/qubit-progress) |
