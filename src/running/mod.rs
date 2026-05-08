/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Running Progress
//!
//! Provides helpers for reporting running progress from a separate loop.
//!
//! # Author
//!
//! Haixing Hu

mod running_progress_loop;
mod running_progress_notifier;
mod running_progress_points;
mod running_progress_signal;
mod scoped_running_progress;

pub use running_progress_loop::RunningProgressLoop;
pub use running_progress_notifier::RunningProgressNotifier;
pub use running_progress_points::RunningProgressPoints;
pub use scoped_running_progress::ScopedRunningProgress;
