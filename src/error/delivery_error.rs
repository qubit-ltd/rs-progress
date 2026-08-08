// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Errors associated with one concrete Event delivery attempt.
// qubit-style: allow source-test-pair

use std::error::Error;
use std::fmt;

use crate::Event;
use crate::error::ReporterError;

/// Failure while delivering one complete Event to a reporter.
#[derive(Clone, Debug)]
pub struct DeliveryError {
    event: Box<Event>,
    source: ReporterError,
}

impl DeliveryError {
    /// Creates a delivery error retaining the failed Event and sink error.
    pub(crate) fn new(event: Event, source: ReporterError) -> Self {
        Self {
            event: Box::new(event),
            source,
        }
    }

    /// Returns the complete Event whose delivery failed.
    #[must_use]
    pub const fn event(&self) -> &Event {
        &self.event
    }

    /// Returns the original reporter error.
    #[must_use]
    pub const fn reporter_error(&self) -> &ReporterError {
        &self.source
    }

    /// Consumes the error and returns its failed Event.
    #[must_use]
    pub fn into_event(self) -> Event {
        *self.event
    }

    /// Consumes the error and returns its reporter error.
    #[must_use]
    pub fn into_reporter_error(self) -> ReporterError {
        self.source
    }

    /// Consumes the error and returns the Event with its reporter error.
    #[must_use]
    pub fn into_parts(self) -> (Event, ReporterError) {
        (*self.event, self.source)
    }
}

impl fmt::Display for DeliveryError {
    /// Formats the failed Event identity and reporter error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "delivery of {} event for operation {} sequence {} failed: {}",
            self.event.phase().as_str(),
            self.event.operation_id(),
            self.event.sequence(),
            self.source,
        )
    }
}

impl Error for DeliveryError {
    /// Returns the reporter error as the cause.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
