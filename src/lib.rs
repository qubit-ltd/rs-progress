/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Generic progress reporting data model and reporter abstractions.
//!
//! This crate models progress as immutable events carrying lifecycle phase,
//! optional stage information, counters, and timing.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod model;
pub mod reporter;
pub mod runtime;
