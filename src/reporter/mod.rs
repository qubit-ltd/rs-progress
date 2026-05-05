/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Progress reporter trait and built-in implementations.

mod impls;
mod progress_reporter;

pub use impls::{
    LoggerProgressReporter,
    NoOpProgressReporter,
    WriterProgressReporter,
};
pub use progress_reporter::ProgressReporter;
