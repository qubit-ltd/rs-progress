# Qubit Progress

[![Rust CI](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-progress/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-progress/coverage-badge.json)](https://qubit-ltd.github.io/rs-progress/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-progress.svg?color=blue)](https://crates.io/crates/qubit-progress)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-progress` is a lifecycle-safe progress-reporting library for one long-running operation. It sends complete, immutable events to a `Reporter`: each event contains the operation phase, elapsed time, stable metric metadata, and the latest dynamic counts.

## Why this crate?

Progress reporting is often coupled to a terminal progress bar or scattered across a copy loop as ad-hoc counters. That makes it difficult to send the same state to logs, JSON, a UI, or telemetry; it also makes consumers reconstruct state from deltas. Threaded work adds another problem: the code that owns the operation and the code that changes the counters are usually different.

This crate separates the two kinds of state. Configure each `Metric`—its stable ID, display name, and optional total—once at startup. Each report closure supplies only the current counts. The crate validates the snapshot and delivers a self-contained `Event`. Its consuming terminal methods permit at most one terminal event and prevent later reports in safe Rust; dropping or unwinding before a terminal call can still abandon an operation.

## Installation

```toml
[dependencies]
qubit-progress = "0.6"
```

Enable `json-lines` for `JsonLinesReporter` or `log` for `LogReporter`.

## Basic use: copy a directory of files

Suppose an import command copies a known list of files. The operation declares the number of files once; after each `std::fs::copy`, it reports the latest completed and successful counts. `TextReporter` writes one complete line per event to standard error, but the same `Progress` code works with a custom reporter.

```rust
use std::{fs, io};
use qubit_progress::{Metric, Progress, TextReporter};

fn copy_files(files: &[(&str, &str)]) -> Result<(), Box<dyn std::error::Error>> {
    let reporter = TextReporter::new(io::stderr());
    let mut progress = Progress::builder(&reporter)
        .metric(Metric::new("files", "Files").total(files.len() as u64))
        .start()?;

    for (index, (source, destination)) in files.iter().enumerate() {
        fs::copy(source, destination)?;
        let completed = index as u64 + 1;
        progress.report(|snapshot| {
            snapshot.metric("files", |counts| {
                counts.completed(completed).succeeded(completed);
            });
        })?;
    }

    progress.finish(|snapshot| {
        snapshot.metric("files", |counts| {
            let total = files.len() as u64;
            counts.completed(total).succeeded(total);
        });
    })?;
    Ok(())
}
```

`Started`, every `Running` event, and `Succeeded` all carry the configured total. The application never repeats that fixed configuration, and a reporter never needs a previous event to understand the current state.

## Worker-thread use: automatically report shared copy state

For parallel or worker-driven work, let `Progress` own a scoped background reporter. The snapshot closure reads the shared counters, while workers call the cloneable `Notifier` after changing them. With `Duration::ZERO`, notifications are coalesced and trigger reports without polling. Call `stop()` before the terminal event: the scoped exclusive borrow prevents manual reporting or termination while the background reporter is active.

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
    .metric(Metric::new("files", "Files").total(2))
    .start()?;
let copied = Arc::new(Mutex::new(0_u64));

thread::scope(|scope| -> Result<(), qubit_progress::ProgressError> {
    let copied_for_snapshot = Arc::clone(&copied);
    let auto = progress.spawn_auto_reporter(scope, move |snapshot| {
        let completed = *copied_for_snapshot.lock().expect("copy counter mutex poisoned");
        snapshot.metric("files", |counts| {
            counts.completed(completed).succeeded(completed);
        });
    });

    let notifier = auto.notifier();
    let copied_for_worker = Arc::clone(&copied);
    let worker = scope.spawn(move || {
        // Perform one copy here, then publish the new shared state.
        *copied_for_worker.lock().expect("copy counter mutex poisoned") = 2;
        notifier.notify();
    });
    worker.join().expect("copy worker panicked");
    auto.stop()?;
    Ok(())
})?;

progress.finish(|snapshot| {
    snapshot.metric("files", |counts| {
        counts.completed(2).succeeded(2);
    });
})?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Use a positive interval when a periodic heartbeat is more useful than immediate notifications. In this mode, `notify()` is a no-op so workers avoid synchronization work; the background reporter wakes only for its timer or `stop()`.

## Next steps

Read the [user guide](doc/user_guide.md) for the lifecycle model, validation rules, scheduling, automatic reporting, reporters, and error handling. API-level details are available on [docs.rs](https://docs.rs/qubit-progress).

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./align-ci.sh` to format code and
`./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-progress](https://github.com/qubit-ltd/rs-progress)
