// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for README consistency.

/// English README text.
const README_EN: &str = include_str!("../../README.md");
/// Chinese README text.
const README_ZH: &str = include_str!("../../README.zh_CN.md");
/// English user guide text.
const USER_GUIDE_EN: &str = include_str!("../../doc/user_guide.md");
/// Chinese user guide text.
const USER_GUIDE_ZH: &str = include_str!("../../doc/user_guide.zh_CN.md");

#[test]
fn test_readmes_describe_qubit_progress() {
    assert!(README_EN.contains("Qubit Progress"));
    assert!(README_EN.contains("ProgressEvent"));
    assert!(README_EN.contains("Progress"));
    assert!(README_EN.contains("ProgressStage"));
    assert!(README_EN.contains("RunningProgressGuard"));
    assert!(README_EN.contains("RunningProgressPointHandle"));
    assert!(README_EN.contains("RunningProgressStatus"));
    assert!(README_EN.contains("background reporter thread"));
    assert!(README_EN.contains("qubit-progress"));
    assert!(!README_EN.contains("Project Layout"));

    assert!(README_ZH.contains("Qubit Progress"));
    assert!(README_ZH.contains("ProgressEvent"));
    assert!(README_ZH.contains("Progress"));
    assert!(README_ZH.contains("ProgressStage"));
    assert!(README_ZH.contains("RunningProgressGuard"));
    assert!(README_ZH.contains("RunningProgressPointHandle"));
    assert!(README_ZH.contains("RunningProgressStatus"));
    assert!(README_ZH.contains("后台汇报线程"));
    assert!(README_ZH.contains("qubit-progress"));
    assert!(!README_ZH.contains("项目结构"));
}

#[test]
fn test_readmes_show_file_copy_event_consumption() {
    for readme in [README_EN, README_ZH] {
        assert!(readme.contains("StderrProgressReporter"));
        assert!(readme.contains("std::fs::copy"));
        assert!(readme.contains("report_running_if_due"));
        assert!(readme.contains("ProgressReporter"));
    }
}

#[test]
fn test_user_guides_explain_file_copy_problem_and_solution() {
    assert!(USER_GUIDE_EN.contains("## Why Qubit Progress?"));
    assert!(USER_GUIDE_EN.contains("## How It Works"));
    assert!(USER_GUIDE_EN.contains("## Quick Start: Copy Files in a CLI"));
    assert!(USER_GUIDE_ZH.contains("## 为什么需要 Qubit Progress？"));
    assert!(USER_GUIDE_ZH.contains("## 如何解决"));
    assert!(USER_GUIDE_ZH.contains("## 快速开始：在命令行中批量复制文件"));
}
