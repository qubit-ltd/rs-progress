// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Private state shared by automatic reporter components.

mod auto_reporter_inner;
mod worker;

pub(in crate::auto_reporter) use auto_reporter_inner::AutoReporterInner;
pub(crate) use worker::spawn;
pub(in crate::auto_reporter) use worker::wake;
