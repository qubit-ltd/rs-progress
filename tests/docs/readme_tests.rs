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
    assert!(README_EN.contains("Progress"));
    assert!(README_EN.contains("ProgressStage"));
    assert!(README_EN.contains("RunningProgressLoop"));
    assert!(README_EN.contains("RunningProgressNotifier"));
    assert!(README_EN.contains("ScopedRunningProgress"));
    assert!(README_EN.contains("RunningProgressPointHandle"));
    assert!(README_EN.contains("background reporter thread"));
    assert!(README_EN.contains("qubit-progress"));
    assert!(!README_EN.contains("Project Layout"));

    assert!(README_ZH.contains("Qubit Progress"));
    assert!(README_ZH.contains("ProgressEvent"));
    assert!(README_ZH.contains("Progress"));
    assert!(README_ZH.contains("ProgressStage"));
    assert!(README_ZH.contains("RunningProgressLoop"));
    assert!(README_ZH.contains("RunningProgressNotifier"));
    assert!(README_ZH.contains("ScopedRunningProgress"));
    assert!(README_ZH.contains("RunningProgressPointHandle"));
    assert!(README_ZH.contains("后台汇报线程"));
    assert!(README_ZH.contains("qubit-progress"));
    assert!(!README_ZH.contains("项目结构"));
}
