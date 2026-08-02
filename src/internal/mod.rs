// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal lifecycle state shared by progress and metric handles.

mod operation_gate;
mod operation_lifecycle;
mod operation_state;
mod update_guard;

pub use operation_lifecycle::OperationLifecycle;
pub(crate) use operation_state::OperationState;
pub(crate) use update_guard::UpdateGuard;
