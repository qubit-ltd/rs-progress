// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal lifecycle state shared by progress and metric handles.

mod atomic_u8_like;
mod atomic_usize_like;
mod finish_guard;
mod gate_lifecycle;
mod operation_gate;
mod operation_lifecycle;
mod operation_state;
mod progress_core;
mod std_scheduler;
mod update_guard;
mod yield_like;

pub(crate) use finish_guard::FinishGuard;
pub use operation_lifecycle::OperationLifecycle;
#[doc(hidden)]
#[cfg(coverage)]
pub use operation_state::__coverage_internal;
pub(crate) use operation_state::OperationState;
pub(crate) use progress_core::build_event;
pub(crate) use progress_core::metric_snapshots;
pub(crate) use progress_core::validate_finish;
pub(crate) use update_guard::UpdateGuard;
