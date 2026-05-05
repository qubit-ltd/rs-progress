/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `format.rs`.

use std::time::Duration;

use super::format_duration;

#[test]
fn test_format_duration_handles_sub_second_and_seconds() {
    assert_eq!(format_duration(Duration::from_millis(0)), "0ms");
    assert_eq!(format_duration(Duration::from_millis(42)), "42ms");
    assert_eq!(format_duration(Duration::from_millis(1_500)), "1.500s");
}

#[test]
fn test_format_duration_handles_minutes_and_hours() {
    assert_eq!(format_duration(Duration::from_secs(61)), "1m 1s");
    assert_eq!(format_duration(Duration::from_secs(3_661)), "1h 1m 1s");
}
