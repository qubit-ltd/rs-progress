// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Signal sent to a running progress loop.
pub(crate) enum RunningProgressSignal {
    /// A worker reached an implementation-defined running progress point.
    RunningPoint,
    /// The operation is complete and the loop should stop.
    Stop,
}
