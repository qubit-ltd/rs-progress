# Qubit Progress 用户指南

`qubit-progress` 表示一次长耗时操作，并把完整事件交给上报器
（`Reporter`）。请先通过 [README](../README.zh_CN.md) 了解文件复制示例；
本指南用于选择指标、安排上报，以及集成工作线程和输出目标。

[English](user_guide.md)

## 选择特性

默认特性集不引入可选依赖。应用只需开启实际使用的集成：

| Feature | 提供的能力 |
| --- | --- |
| `serde` | 为 `Event`、`Phase`、`Stage` 和 `MetricSnapshot` 实现 `Serialize` 与 `Deserialize` |
| `json-lines` | 提供 `JsonLinesReporter`，并同时开启 `serde` |
| `log` | 通过 `log` 门面提供 `LogReporter` |

## 核心模型

- `ProgressBuilder` 在启动前收集上报器、上报间隔、指标、可选的 `Stage` 和不可变的 operation attributes。
- 指标定义（`Metric`）是稳定元数据：机器可读 ID、显示名称和可选总量。
- `Progress` 持有每个指标的动态状态。通过 `Progress::metric` 取得可克隆的指标句柄（`MetricHandle`）后，调用方只需执行状态转换，无需维护外部计数器。
- `Event` 是不可变的完整观察结果，上报器无需依赖先前事件重建状态。已启用的操作会获得进程内唯一的 `operation_id`；`Started` 的 `sequence` 为零，后续序号按投递尝试递增，包括失败的尝试。`OperationAttributes` 会把可选的字符串关联元数据附加到每个事件。

生命周期为 `Started → Running* → Succeeded | Failed | Cancelled`。`finish`、`finish_recoverable`、`finish_unchecked`、`fail` 和 `cancel` 会消费 `Progress`，因此安全 Rust 中最多只能发送一个终态事件。`finish()` 要求 active 为零且所有已知总量都完成；它返回不带生命周期的 `FinishError`，可以直接使用 `?` 传播。只有需要修复指标并重试时才使用 `finish_recoverable()`。只有在业务上允许以未完成状态成功结束时，才使用 `finish_unchecked()`。终态前丢弃对象或 unwind 仍可能使操作没有终态事件。

## 启动并更新指标

代码中应使用稳定的指标 ID，输出时则使用便于阅读的名称。总量可以省略；
如果已知，应只配置一次。每个操作至少需要一个指标；指标 ID 和名称不能是
空白，且 ID 必须唯一。`Stage` 的 ID 和名称同样不能是空白；如果设置位置，
它必须位于从 1 开始的 `1..=total` 范围内。

下面的操作先上报一批正在处理的数据，随后完成它。

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
files.complete(2)?;
progress.report()?;

files.start(4)?;
files.succeed(4)?;
let elapsed = progress.finish()?;
# let _ = elapsed;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`start()` 返回 `StartError`，用于表示配置无效、操作 ID 耗尽或 `Started` 投递被拒绝。
`report()` 将当前指标快照作为 `Running` 发送并返回 `EmissionError`。终态投递失败时，
`TerminalError` 同时保留耗时和失败事件的投递错误。

事件投递采用“最多一次”语义。库不会自动重试上报器错误，而是将错误返回给
调用方。如果业务层选择重试，上报器可能已经接受事件但随后返回错误，因此重试
可能产生重复事件；需要幂等性的 sink 应使用应用生成的进程实例 ID 作为命名空间，并按
`(process_instance_id, operation_id, sequence)` 去重。应用应在一次事件重放期间保持该 ID
不变，为并发进度实例和进程重启分配新 ID。它可放入 `OperationAttributes` 或外部信封。
只有任务 ID 和 attempt 足以唯一标识进程及进度实例时，才能用它们代替实例 ID，不能默认
它们足够唯一。

重放 `DeliveryError::event()` 或 `DeliveryError::into_parts()` 返回的原始 `Event` 会保留
原键。重新调用 `report()` 会生成新序号，重新启动会生成新 operation ID；进程重启后该 ID
还可能重复。这些调用不能自动识别为同一业务事件的重放，sink 应自行负责幂等策略。

有意跳过指标校验时使用 `finish_unchecked()`；正常成功结束使用
`finish()`：

```rust
files.start(10)?;
files.succeed(10)?;
progress.finish()?;
```

`finish()` 会拒绝仍有 active 工作的指标，也会拒绝 `completed != total` 的已知总量指标。
校验失败时 `finish()` 返回包含 `CompletionError` 和校验前采样耗时的
`FinishError::Incomplete`，此时操作已经被消费。`FinishError::elapsed()` 对校验失败和终态
投递失败都返回同一个采样值。该值是结束尝试开始时采样的操作已逝时间，早于冻结指标
更新、完成校验和终态投递；它不是 `finish()` 调用自身的耗时，也不包含随后等待终态投递的时间。
需要修复指标后重试时使用 `finish_recoverable()`，其
`RecoverableFinishError::Incomplete` 才会带回仍可使用的 `Progress`。只有 terminal
错误表示已经尝试终态投递且操作不可恢复。
指南中的第一个示例将十个文件计入成功、失败和未分类完成；`finish()` 检查完成总数，
并不要求所有文件都成功。

## 指标生命周期与校验

`start(count)` 把无符号数量从未开始移动到 active。`complete`、`succeed`、`fail` 和 `cancel` 把无符号数量从 active 移动到各自的完成状态。`completed` 是未分类完成、成功、失败和取消之和；指标是单向的，不提供 `rollback()`。任何计数都不能变为负数；已知总量时，`active + completed` 不能超过总量。

`completed` 包含未分类完成、成功、失败和取消。每次转换均在 CAS 临界区中校验后
提交，因此每个发送出的指标快照都内部一致。包含多个指标的事件只保证每个指标
快照各自一致；运行中的多个指标之间不是全局原子视图，操作关闭后的终态事件则
保持稳定。不要用计数表达阶段信息：在启动时使用 `Stage`，用 `set_stage` 替换后续
事件的阶段，或用 `clear_stage` 清除它。

### 使用原子复合更新

一个业务操作同时改变多个计数时，应使用 `MetricDelta`。其中所有字段都是增量，
不是调用方根据旧快照计算出的绝对目标值；整个 delta 会在一个线性化点校验并提交：

```rust
let completed = 10;
let succeeded = 8;
items.apply_delta(
    MetricDelta::new()
        .started(completed)
        .succeeded(succeeded)
        .unclassified(completed - succeeded),
)?;
```

并发的增量提交不会互相覆盖；如果某个 delta 在线性化点尝试完成超过现有 active 的
数量，会返回 `MetricError::InsufficientActive` 且不修改状态。`MetricSnapshot::unclassified()`
可用于读取没有 outcome 分类的终态数量。

生命周期仍然是单向的：`started` 增加 active，每个终态字段都把 active
转移到且仅转移到一种终态。在同一个 delta 内，`started` 会先计入再处理终态字段，
因此可以原子地提交“开始并完成”的批次。并发 delta 的串行化顺序会影响结果：如果
负责增加 active 的 delta 尚未线性化，只有终态字段的 delta 可能返回
`MetricError::InsufficientActive`；需要此顺序时应重试或在业务侧协调两次更新。

## 使用 operation attributes 关联事件

在 `start()` 前通过 `ProgressBuilder` 设置字符串属性：

```rust
let progress = Progress::builder(&reporter)
    .attribute("tenant_id", "tenant-7")
    .attribute("trace_id", "trace-abc")
    .metric(Metric::new("files", "Files"))
    .start()?;
```

启动后 attributes 不可修改，并会附加到每个事件。它们按稳定顺序输出到文本和 JSON；
由于上报器会原样输出，请勿放置密码或令牌。

## 结束每次操作

类型系统能够阻止第二个终态事件，但无法在业务操作返回错误时替应用选择结果。
返回前应发送 `Succeeded`、`Failed` 或 `Cancelled`：

```rust
use std::fs;
use qubit_progress::{Metric, Progress, Reporter};

fn copy_one(
    reporter: &dyn Reporter,
    source: &str,
    destination: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let progress = Progress::builder(reporter)
        .metric(Metric::new("files", "文件").total(1))
        .start()?;
    let files = progress.metric("files").expect("configured metric must exist");

    let work_result = (|| -> Result<(), Box<dyn std::error::Error>> {
        files.start(1)?;
        fs::copy(source, destination)?;
        files.succeed(1)?;
        Ok(())
    })();

    match work_result {
        Ok(()) => {
            progress.finish()?;
            Ok(())
        }
        Err(work_error) => {
            progress.fail()?;
            Err(work_error)
        }
    }
}
```

如果操作由调用方或用户主动停止，应改用 `cancel()`。上例中，
`progress.fail()?` 会使终态投递错误优先于业务错误返回。需要同时保留两类错误时，
应在尝试发送终态前记录业务错误，或把两个错误组合起来。

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
    records.start(1)?;
    // 处理一条记录。
    records.succeed(1)?;
    progress.report_if_due()?;
}
progress.finish()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

间隔为零时，每次 `report_if_due()` 都到期。调度使用相对耗时，因此包括
`Duration::MAX` 在内的间隔都能启动；该值只是不再产生未来的自动到期点。上报器失败
会消耗一个投递序号并重置下次截止时间；因此序号空洞表示投递尝试失败，而不是缺少
状态转换。

## writer 缓冲与部分写入失败

内置文本和 JSON Lines 上报器会把完整记录交给配置的 `Write` 实现。`report()` 成功表示
writer 接受了记录，不会自动调用 `flush()`。`into_inner()` 只取回 writer。停止上报并确保
writer 不再被共享使用后，应用应显式 flush 并处理结果：

```rust
use std::io::{BufWriter, Write};
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(BufWriter::new(Vec::<u8>::new()));
let mut progress = Progress::builder(&reporter)
    .metric(Metric::new("tasks", "Tasks").total(1))
    .start()?;
let tasks = progress.metric("tasks").expect("configured metric must exist");
tasks.start(1)?;
tasks.succeed(1)?;
progress.report()?;
progress.finish()?;
let mut writer = reporter.into_inner()?;
writer.flush()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

写入 `File` 时也应处理 `flush()` 错误。flush 成功不保证数据已持久化到磁盘；需要持久化的
应用还应按自身要求执行文件同步并遵守业务事务策略。`write_all` 出错前可能已经写入一段
前缀，该前缀不会回滚，后续记录也不保证从新行开始。继续写入前，应重新打开或替换目标，
或按存储格式支持的恢复流程重新同步。上报器不会自动重试。

## 自动上报工作线程的状态

`spawn_auto_reporter` 会上报由工作线程更新的指标状态。它在整个 `Progress` 操作期间
只启动一个有作用域的后台线程并独占可变借用；任意数量的工作线程共享指标句柄和
通知器。返回的 `AutoReporter` 提供可克隆的 `ProgressNotifier` 和 `AutoReporterStatus`。

```rust
use std::{thread, time::Duration};
use qubit_progress::{Metric, Progress, TextReporter};

let reporter = TextReporter::new(std::io::stderr());
let mut progress = Progress::builder(&reporter)
    .interval(Duration::ZERO)
    .metric(Metric::new("files", "Files").total(3))
    .start()?;
let files = progress.metric("files").expect("configured metric must exist");

thread::scope(|scope| -> Result<(), qubit_progress::AutoReporterError> {
    let auto = progress.spawn_auto_reporter(scope);
    let status = auto.status();
    let notifier = auto.notifier();
    let worker = scope.spawn(move || {
        files.start(3).expect("metric update must succeed");
        // 在这里执行复制。
        files.succeed(3).expect("metric update must succeed");
        notifier.notify();
    });
    worker.join().expect("copy worker panicked");
    auto.stop()?;
    assert!(!status.is_failed());
    Ok(())
})?;

progress.finish()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

零间隔下，`notify()` 会把重复通知合并成最多一个待发送上报。正间隔下后台线程
按最小间隔发送心跳，`notify()` 是无操作，从而避免工作线程承担同步开销。
`notify()` 在 `stop()` 后无害。调用终态方法前必须调用 `stop()` 并处理其结果。
`AutoReporter` 存活期间，独占借用会阻止手工上报、修改阶段和结束操作。

当前自动驱动器为每个启用的 `Progress` 操作创建一个有作用域的 `std::thread`，
尚未提供 `AsyncProgress` 专用的自动调度器。使用 `AsyncReporter` 时，由应用运行时
安排并 await `AsyncProgress::report_if_due_async()`；使用同步 `Reporter` 时调用
`Progress::report_if_due()`。同步回调会直接执行，应用应负责避免阻塞型输出占用异步
运行时的工作线程。定时、取消和背压策略由应用负责。

后台上报器因错误或 panic 退出后，`AutoReporterStatus::is_failed()` 会变为 `true`。
工作线程可以观察克隆的 `AutoReporterStatus`，以便提前停止高成本工作；但 `stop()` 才是最终依据：
它会 join 后台线程，返回校验或上报器错误，或返回结构化的后台 panic 信息。
直接丢弃句柄仍会停止并回收后台线程，但不会传播 panic。

## 异步上报器

`AsyncReporter::report()` 返回可发送的装箱 Future，`AsyncProgress` 提供与同步生命周期
对应的 `start_async()`、`report_async()`、到期上报和异步终态方法。crate 不依赖特定异步
运行时；调用方在自己的 executor 中 await 这些方法。Future 返回 `Ok(())` 表示上报器已
接收事件。

上报器方法返回装箱 Future 类型 `ReportFuture<'a>`：

```rust
impl AsyncReporter for RecordingSink {
    fn report<'a>(&'a self, event: &'a Event) -> ReportFuture<'a> {
        Box::pin(async move {
            self.0.lock().expect("recording lock must work").push(event.clone());
            Ok(())
        })
    }
}
```

操作先 await 启动，工作指标转换本身同步执行，然后 await 到期上报与终态上报。业务失败时，
先尝试投递失败终态，同时保留原始业务错误：

```rust
async fn run_batch(reporter: &dyn AsyncReporter, fail_work: bool) -> Result<(), Box<dyn Error>> {
    let mut progress = AsyncProgress::builder(reporter)
        .metric(Metric::new("items", "Items").total(1))
        .start_async()
        .await?;
    let items = progress.metric("items").expect("configured metric must exist");
    items.start(1)?;

    if fail_work {
        let work_error = io::Error::other("demonstration work failure");
        items.fail(1)?;
        if let Err(report_error) = progress.fail_async().await {
            eprintln!("terminal reporting also failed: {report_error}");
        }
        return Err(Box::new(work_error));
    }

    items.succeed(1)?;
    progress.report_if_due_async().await?;
    progress.finish_async().await?;
    Ok(())
}
```

若调用方取消正在等待的上报 Future，投递结果未知。运行中事件会在等待投递前消耗序号，
终态操作在 Future 被轮询后即被消费。除非上报器自身提供去重，否则不要自动重试终态事件。

[examples/async_reporting.rs](../examples/async_reporting.rs) 提供了完整的独立示例：它实现
只记录内存事件的 `RecordingSink`，并依次 await 启动、到期上报和成功结束。运行命令为：

```sh
cargo run --locked --example async_reporting
```

输出如下：

```text
success: Started -> Running -> Succeeded; work error: Started -> Failed
```

示例使用默认零间隔，因此 `report_if_due_async().await` 会立即上报。正间隔下，应用应由
自己的运行时周期调用 `AsyncProgress::report_if_due_async().await`。同步对应方法是
`Progress::report_if_due()`，无需 `.await`。`RecordingSink` 只在内存中记录事件，不是持久化
存储。实际 sink 应 await 存储操作，并用 `ReporterError::new` 映射存储错误，使成功路径的
错误通过 `?` 传播。业务失败路径保留并返回业务错误，同时单独记录终态投递错误。普通的
提前返回不会自动发送终态事件；在业务结果明确处安排 `fail_async()` 或 `cancel_async()`。
指标校验失败和投递失败是不同情况：完成校验可以在尝试终态投递前失败；终态投递错误则表示
操作已被消费且投递已经尝试。

对于 `report_if_due_async()`，已轮询的到期 `Running` Future 会消耗序号。若投递正常完成或
返回错误，下次截止时间从完成时重新计算。若 Future 一直 pending 后被取消，截止时间保持
不变，因此下一次调用可能立即到期；操作仍可继续使用。丢弃未轮询的 Future 不消耗序号，
也不改变截止时间。禁用操作不会发送事件，调度状态保持不变。若应用因 timeout 取消上报且
需要限制重试频率，应自行执行 backoff；crate 不会自动重试。

| 结果 | 序号 | 下次截止时间 | 操作 |
| --- | --- | --- | --- |
| 未到期或已禁用 | 不变 | 不变 | 可继续使用 |
| 到期上报完成（成功或错误） | 已消耗 | 从完成时重置 | 可继续使用 |
| 已轮询的到期上报 pending 后取消 | 已消耗 | 不变；下次可能立即到期 | 可继续使用 |
| 丢弃未轮询的 `Running` Future | 不变 | 不变 | 可继续使用 |

## 禁用操作

`Reporter::is_enabled()` 的结果由 `start()` 采样一次。禁用操作仍会校验固定配置并
维护指标状态，但不发送事件，也不启动自动上报线程。这样即使业务代码始终执行
上报路径，输出目标关闭时的成本也很低。

调用方需要明确禁用输出的上报器时，可以使用 `NoopReporter`。自定义上报器可以
覆盖 `is_enabled()`，把启用状态连接到应用配置。

## 上报器与结构化输出

自定义上报器需要实现 `Reporter`、消费 `&Event`，并满足 `Send + Sync`。
内置上报器包括：

- `TextReporter<W>`：向任意 `Write + Send` 目标写入每个事件的一行可读文本。
- `NoopReporter`：禁用事件。
- `JsonLinesReporter<W>`：开启 `json-lines` 后可用，每行写入一个完整 JSON 事件。
- `LogReporter`：开启 `log` 后可用，在 info 级别写入事件的 `Debug` 表示。

文本输出会保留可打印 Unicode（包括普通中文）的可读形式。metadata 中的换行、回车、制表符
等控制字符会被转义，因此每个事件仍只占一行。

满足 `Fn(&Event) -> Result<(), ReporterError> + Send + Sync` 的闭包也会自动实现
`Reporter`，通常是最简短的自定义集成方式。

JSON Lines 适合日志采集器和后处理，因为每一行都是完整事件：

```json
{"operation_id":42,"sequence":1,"phase":"running","stage":null,"metrics":[{"id":"files","name":"文件","total":3,"completed":1,"active":1,"succeeded":1,"failed":0,"cancelled":0}],"elapsed":"250ms"}
```

耗时使用整数加最大精确单位编码，单位包括 `h`、`m`、`s`、`ms`、`us` 和
`ns`。反序列化时会执行与运行期构造相同的公共事件不变量校验。

## 错误与关闭

`start()` 返回 `StartError`，运行中上报返回 `EmissionError`，指标句柄的状态转换
直接返回 `MetricError`。不要忽略这些错误：状态无效表示事件没有发送，而输出目标
错误表示本次投递失败。`finish()` 返回不带生命周期的 `FinishError`，可以直接使用
`?` 传播。`elapsed()` 返回结束尝试开始时采样的操作已逝时间，早于冻结指标更新、完成校验
（如适用）和终态投递；它不是 `finish()` 调用自身的耗时，也不包含随后等待终态投递的时间。
异步方法会在 Future 被 poll 并到达终态逻辑时采样，而不是在创建 Future 时采样。需要修复并重试时使用 `finish_recoverable()` 和
`RecoverableFinishError`。

终态方法的返回类型如下：

| 方法 | 返回值 |
| --- | --- |
| `finish` | `Result<Duration, FinishError>` |
| `finish_recoverable` | `Result<Duration, RecoverableFinishError<'reporter>>` |
| `finish_unchecked`、`fail`、`cancel` | `Result<Duration, TerminalError>` |
| `finish_async` | `.await` 后为 `Result<Duration, FinishError>` |
| `finish_unchecked_async`、`fail_async`、`cancel_async` | `.await` 后为 `Result<Duration, TerminalError>` |

`FinishError` 保留完成校验错误与终态投递错误的区别；`TerminalError` 会在终态投递失败时
保留耗时。自动上报场景下，`AutoReporter::stop()` 返回 `AutoReporterError`：`Emission`
变体携带普通投递失败，`Panicked` 变体保留后台 panic 载荷。`Drop` 不传播 panic，只适合
尽力清理。

启用 `serde` 特性后，反序列化 `Stage`、`MetricSnapshot` 或 `Event` 时会校验与在线进度操作相同的元数据和计数不变量。反序列化错误表示外部进度数据无效，而不是可恢复的事件状态。

## 进一步参考

每种类型、错误变体、上报器特性和序列化细节，请参阅
[docs.rs](https://docs.rs/qubit-progress) 生成的 API 文档。
