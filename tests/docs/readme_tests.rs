/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for README consistency.

/// English README text.
const README_EN: &str = include_str!("../../README.md");
/// Chinese README text.
const README_ZH: &str = include_str!("../../README.zh_CN.md");

#[test]
fn test_readmes_describe_qubit_progress() {
    assert!(README_EN.contains("Qubit Progress"));
    assert!(README_EN.contains("ProgressEvent"));
    assert!(README_EN.contains("ProgressRun"));
    assert!(README_EN.contains("ProgressStage"));
    assert!(README_EN.contains("qubit-progress"));
    assert!(!README_EN.contains("Project Layout"));

    assert!(README_ZH.contains("Qubit Progress"));
    assert!(README_ZH.contains("ProgressEvent"));
    assert!(README_ZH.contains("ProgressRun"));
    assert!(README_ZH.contains("ProgressStage"));
    assert!(README_ZH.contains("qubit-progress"));
    assert!(!README_ZH.contains("项目结构"));
}
