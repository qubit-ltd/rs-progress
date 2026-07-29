# Qubit Progress

[![Rust CI](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-progress/coverage-badge.json)](https://qubit-ltd.github.io/rs-progress/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-progress.svg?color=blue)](https://crates.io/crates/qubit-progress)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-progress` 是一个面向单次长耗时操作、具有生命周期安全保证的进度上报库。它向上报器（`Reporter`）发送完整且不可变的事件：每个事件都包含操作阶段、耗时、稳定的指标元数据和最新动态计数。

## 这个库解决什么问题？

进度上报常被绑死在终端进度条上，或以零散计数器散落在复制循环中。这样既难以把同一状态同时发送给日志、JSON、UI 或遥测系统，也会迫使消费者根据增量重建状态。进入多线程后，操作所有者和更新计数的工作线程又常常不是同一段代码。

本库把这两类状态分开：启动时仅配置一次 `Metric` 的稳定 ID、显示名称和可选总量；随后由 `Progress` 持有动态计数，可克隆的指标句柄以经过校验的生命周期转换更新它们，每个事件都读取一个内部一致的快照。终态方法会消费 `Progress`，因此在安全 Rust 中至多发送一个终态事件，且不能在终态后继续上报；但若在调用终态方法前丢弃对象或发生 unwind，操作仍可能没有终态事件。

## 安装

```toml
[dependencies]
qubit-progress = "0.6"
```

需要序列化或反序列化事件数据时开启 `serde`；需要 `JsonLinesReporter` 时开启
`json-lines`（它会同时开启 `serde`）；需要 `LogReporter` 时开启 `log`。

## 常规用法：复制一组文件

假设导入命令要复制一组已知文件。操作只在启动时声明文件总数；每复制一个文件前将其标为进行中，成功后再标为成功并上报最新计数。`TextReporter` 将每个事件写成一行到标准错误；替换上报器后，同一段 `Progress` 代码也可以服务于其他输出目标。

```rust
use std::{fs, io};
use qubit_progress::{Metric, Progress, TextReporter};

fn copy_files(files: &[(&str, &str)]) -> Result<(), Box<dyn std::error::Error>> {
    let reporter = TextReporter::new(io::stderr());
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("files", "文件").total(files.len() as u64))
        .start()?;

    let files_metric = progress.metric("files").expect("configured metric must exist");
    for (source, destination) in files {
        files_metric.start(1)?;
        fs::copy(source, destination)?;
        files_metric.succeed(1)?;
        progress.report()?;
    }

    progress.finish()?;
    Ok(())
}
```

`Started`、每个 `Running` 和 `Succeeded` 事件都会携带已配置的总量。业务代码无需重复填写固定配置，上报器也不需要之前的事件即可理解当前状态。

这个示例只展示成功路径。失败或取消时，应在返回前发送对应终态事件；具体模式见
[结束每次操作](doc/user_guide.zh_CN.md#结束每次操作)。

## 工作线程场景：自动上报共享的复制状态

对于并行或由工作线程驱动的任务，可让 `Progress` 拥有一个有作用域的后台上报器。工作线程更新可克隆的指标句柄后调用可克隆的 `Notifier`。使用 `Duration::ZERO` 时，通知会被合并并触发上报，无需轮询。发送终态事件前必须调用 `stop()`：有作用域的独占借用会阻止后台上报器存活期间手工上报或结束操作。

```rust
use std::{thread, time::Duration};
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(std::io::stderr());
let mut progress = Progress::builder(&reporter)
    .interval(Duration::ZERO)
    .metric(Metric::new("files", "文件").total(2))
    .start()?;
let files_metric = progress.metric("files").expect("configured metric must exist");

thread::scope(|scope| -> Result<(), qubit_progress::ProgressError> {
    let auto = progress.spawn_auto_reporter(scope);

    let notifier = auto.notifier();
    let worker = scope.spawn(move || {
        files_metric.start(2).expect("metric update must succeed");
        // 在这里执行复制。
        files_metric.succeed(2).expect("metric update must succeed");
        notifier.notify();
    });
    worker.join().expect("copy worker panicked");
    auto.stop()?;
    Ok(())
})?;

progress.finish()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

若更需要周期性心跳而不是即时通知，可设置正的上报间隔。此时 `notify()` 是无操作，工作线程无需为通知付出同步成本；后台上报器只会因定时器或 `stop()` 被唤醒。

## 下一步

请阅读[用户指南](doc/user_guide.zh_CN.md)，了解生命周期模型、校验规则、调度、自动上报、上报器与错误处理；API 细节见 [docs.rs](https://docs.rs/qubit-progress)。

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
