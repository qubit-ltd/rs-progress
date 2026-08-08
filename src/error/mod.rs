// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Structured errors for progress configuration, state and delivery.

mod auto_reporter_error;
mod completion_error;
mod configuration_error;
mod delivery_error;
mod emission_error;
mod finish_error;
mod metric_error;
mod recoverable_finish_error;
mod reporter_error;
mod start_error;
mod terminal_error;

pub use auto_reporter_error::AutoReporterError;
pub use auto_reporter_error::WorkerPanic;
pub use completion_error::CompletionError;
pub use configuration_error::ConfigurationError;
pub use delivery_error::DeliveryError;
pub use emission_error::EmissionError;
pub use finish_error::FinishError;
pub use metric_error::MetricError;
pub use recoverable_finish_error::RecoverableFinishError;
pub use reporter_error::ReporterError;
pub use start_error::StartError;
pub use terminal_error::TerminalError;
