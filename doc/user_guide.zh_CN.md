# Qubit Progress 用户指南

`qubit-progress` 表示一次长耗时操作，并把完整事件交给上报器
（`Reporter`）。请先通过 [README](../README.zh_CN.md) 了解文件复制示例；
本指南用于选择指标、安排上报，以及集成工作线程和输出目标。

## 选择特性

默认特性集不引入可选依赖。应用只需开启实际使用的集成：

| Feature | 提供的能力 |
| --- | --- |
| `serde` | 为 `Event`、`Phase`、`Stage` 和 `MetricSnapshot` 实现 `Serialize` 与 `Deserialize` |
| `json-lines` | 提供 `JsonLinesReporter`，并同时开启 `serde` |
| `log` | 通过 `log` 门面提供 `LogReporter` |

## 核心模型

- `ProgressBuilder` 在启动前收集上报器、上报间隔、指标和可选的 `Stage`。
- 指标定义（`Metric`）是稳定元数据：机器可读 ID、显示名称和可选总量。
- `Progress` 持有每个指标的动态状态。通过 `Progress::metric` 取得可克隆的指标句柄（`MetricHandle`）后，调用方只需执行状态转换，无需维护外部计数器。
- `Event` 是不可变的完整观察结果，上报器无需依赖先前事件重建状态。已启用的操作会获得进程内唯一的 `operation_id`；`Started` 的 `sequence` 为零，后续序号按投递尝试递增，包括失败的尝试。

生命周期为 `Started → Running* → Succeeded | Failed | Cancelled`。`finish`、`fail` 和 `cancel` 会消费 `Progress`，因此安全 Rust 中最多只能发送一个终态事件。`finish()` 有意保持宽松：`Succeeded` 表示操作已经结束，不表示每个指标都已达到配置的总量；如果成功结束还必须满足 active 为零且所有已知总量都完成，应改用 `finish_checked()`。校验失败时操作会关闭，但不会发送 `Succeeded`。终态前丢弃对象或 unwind 仍可能使操作没有终态事件。

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
files.complete(1)?;
progress.report()?;

files.start(4)?;
files.succeed(4)?;
let elapsed = progress.finish()?;
# let _ = elapsed;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`start()` 会校验全部固定元数据；如果操作已启用，随后发送 `Started`。
`report()` 将当前指标快照作为 `Running` 发送。终态方法返回耗时
`Duration`；终态投递失败时，`TerminalError` 同时保留耗时和底层
`ProgressError`。

事件投递采用“最多一次”语义。库不会自动重试上报器错误，而是将错误返回给
调用方。如果业务层选择重试，上报器可能已经接受事件但随后返回错误，因此重试
可能产生重复事件；需要幂等性的 sink 应使用事件的 `operation_id` 和 `sequence`
进行去重。

宽松成功结束使用 `finish()`；需要检查指标状态时使用
`finish_checked()`：

```rust
files.start(10)?;
files.succeed(10)?;
progress.finish_checked()?;
```

`finish_checked()` 会拒绝仍有 active 工作的指标，也会拒绝
`completed != total` 的已知总量指标。即使校验失败，它仍会消费并关闭操作，
因此应在调用前选择好使用 `finish()` 还是其他终态方法。

## 指标生命周期与校验

`start(count)` 把无符号数量从未开始移动到 active。`complete`、`succeed`、`fail` 和 `cancel` 把无符号数量从 active 移动到各自的完成状态。若要撤销其中一次移动，使用匹配的 `MetricTransition` 调用 `rollback(transition, count)`：终态数量回到 active，而 `MetricTransition::Start` 则把 active 数量回到未开始。任何计数都不能变为负数；已知总量时，`active + completed` 不能超过总量。

`completed` 包含未分类完成、成功、失败和取消。每次转换均在 CAS 临界区中校验后
提交，因此每个发送出的指标快照都内部一致。包含多个指标的事件只保证每个指标
快照各自一致；运行中的多个指标之间不是全局原子视图，操作关闭后的终态事件则
保持稳定。不要用计数表达阶段信息：在启动时使用 `Stage`，用 `set_stage` 替换后续
事件的阶段，或用 `clear_stage` 清除它。

### 注意复合更新的观测边界

每个 `MetricHandle` 方法只完成一次原子状态转换，多个方法调用并不构成事务。
例如，一个已完成的 chunk 中包含不同结果的 item，可以按以下方式记录：

```rust
let completed = 10;
let succeeded = 8;
items.start(completed)?;
items.succeed(succeeded)?;
items.complete(completed - succeeded)?;
```

自动上报器可能在这些调用之间观察到有效的中间快照，例如十个 item 全部处于
active，或只有八个 item 已完成。因此，Running 事件不能被解释为原子业务事务。
如果消费者需要权威的聚合结果，应使用终态事件，或者在 `MetricHandle` 之外同步
业务更新与上报边界。

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

间隔为零时，每次 `report_if_due()` 都到期。无法由 `Instant` 表示的非零间隔会在
`start()` 时被拒绝。上报器失败会消耗一个投递序号并重置下次截止时间；因此序号
空洞表示投递尝试失败，而不是缺少状态转换。

## 自动上报工作线程的状态

`spawn_auto_reporter` 会上报由工作线程更新的指标状态。它启动一个有作用域的
后台线程并独占可变的 `Progress` 借用；返回的 `AutoReporter` 提供可克隆的
`Notifier` 和 `Status`。

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

后台上报器因错误或 panic 退出后，`Status::is_failed()` 会变为 `true`。工作线程
可以观察克隆的 `Status`，以便提前停止高成本工作；但 `stop()` 才是最终依据：
它会 join 后台线程、返回校验或上报器错误，并在调用线程恢复 panic。

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

满足 `Fn(&Event) -> Result<(), ReportError> + Send + Sync` 的闭包也会自动实现
`Reporter`，通常是最简短的自定义集成方式。

JSON Lines 适合日志采集器和后处理，因为每一行都是完整事件：

```json
{"operation_id":42,"sequence":1,"phase":"running","stage":null,"metrics":[{"id":"files","name":"文件","total":3,"completed":1,"active":1,"succeeded":1,"failed":0,"cancelled":0}],"elapsed":"250ms"}
```

耗时使用整数加最大精确单位编码，单位包括 `h`、`m`、`s`、`ms`、`us` 和
`ns`。反序列化时会执行与运行期构造相同的公共事件不变量校验。

## 错误与关闭

大多数非终态操作返回 `Result<(), ProgressError>`。`ProgressError` 会区分校验
失败和上报器返回错误；指标句柄的状态转换会直接返回 `MetricError`。不要忽略
这些错误：状态无效表示事件没有发送，而输出目标错误表示本次投递失败。

终态方法返回 `Result<Duration, TerminalError>`。如果必须可靠记录完成结果，
应检查 `TerminalError`：即使最终事件无法投递，它仍会保留操作耗时。自动上报
场景下，`AutoReporter::stop()` 会返回后台校验或上报器错误，并在调用线程恢复
后台上报线程的 panic。

启用 `serde` 特性后，反序列化 `Stage`、`MetricSnapshot` 或 `Event` 时会校验与在线进度操作相同的元数据和计数不变量。反序列化错误表示外部进度数据无效，而不是可恢复的事件状态。

## 进一步参考

每种类型、错误变体、上报器特性和序列化细节，请参阅
[docs.rs](https://docs.rs/qubit-progress) 生成的 API 文档。

## TODO

- 增加可选的稳定 operation correlation metadata，使共享 Reporter 能够把并发操作
  关联到应用层的 job 或 request。
