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

mod running_progress_guard;
mod running_progress_loop;
mod running_progress_notifier;
mod running_progress_point_handle;
mod running_progress_signal;

pub use running_progress_guard::RunningProgressGuard;
pub(crate) use running_progress_loop::RunningProgressLoop;
pub use running_progress_point_handle::RunningProgressPointHandle;
