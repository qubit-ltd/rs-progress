// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Optional operation stage metadata.

#[path = "stage/stage.rs"]
mod implementation;
#[cfg(feature = "serde")]
mod internal;

pub use implementation::Stage;
