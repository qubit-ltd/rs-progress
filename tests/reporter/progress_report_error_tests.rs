// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `ProgressReportError`.

use std::{
    error::Error,
    io,
};

use qubit_progress::ProgressReportError;

#[test]
fn test_progress_report_error_preserves_io_context() {
    let error = ProgressReportError::from(io::Error::other("output closed"));

    assert!(error.to_string().contains("output closed"));
    assert!(error.source().is_some());
    assert_eq!(error, error.clone());
}
