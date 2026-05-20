/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `ProgressMetric`.

use qubit_progress::ProgressMetric;

#[test]
fn test_progress_metric_stores_id_and_name() {
    let metric = ProgressMetric::new("bytes", "Bytes");

    assert_eq!(metric.id(), "bytes");
    assert_eq!(metric.name(), "Bytes");
}
