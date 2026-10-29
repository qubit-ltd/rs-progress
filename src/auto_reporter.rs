// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Scoped background reporting for one exclusively borrowed progress operation.

// Defines the scoped handle and its lifecycle methods.
#[allow(clippy::module_inception)]
mod auto_reporter;
// Defines the cloneable failure status exposed to workers.
mod auto_reporter_status;
// Defines private shared control state and worker implementation.
mod internal;
// Defines the cloneable notification handle.
mod progress_notifier;

pub use auto_reporter::AutoReporter;
pub use auto_reporter_status::AutoReporterStatus;
pub(crate) use internal::spawn;
pub use progress_notifier::ProgressNotifier;
