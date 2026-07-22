// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Unchecked serde representations validated at model boundaries.

mod progress_event_unchecked;
mod progress_schema_unchecked;
mod progress_stage_unchecked;

pub(crate) use progress_event_unchecked::ProgressEventUnchecked;
pub(crate) use progress_schema_unchecked::ProgressSchemaUnchecked;
pub(crate) use progress_stage_unchecked::ProgressStageUnchecked;
