/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `ProgressPhase`.

use qubit_progress::model::ProgressPhase;

#[test]
fn test_progress_phase_formats_all_variants() {
    assert_eq!(ProgressPhase::Started.as_str(), "started");
    assert_eq!(ProgressPhase::Running.as_str(), "running");
    assert_eq!(ProgressPhase::Finished.as_str(), "finished");
    assert_eq!(ProgressPhase::Failed.as_str(), "failed");
    assert_eq!(ProgressPhase::Canceled.as_str(), "canceled");
    assert_eq!(ProgressPhase::Finished.to_string(), "finished");
}

#[test]
fn test_progress_phase_identifies_terminal_phases() {
    assert!(!ProgressPhase::Started.is_terminal());
    assert!(!ProgressPhase::Running.is_terminal());
    assert!(ProgressPhase::Finished.is_terminal());
    assert!(ProgressPhase::Failed.is_terminal());
    assert!(ProgressPhase::Canceled.is_terminal());
}
