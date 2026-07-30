// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Reporter output and event serialization tests.

use std::{
    io::{
        self,
        Write,
    },
    panic::{
        AssertUnwindSafe,
        catch_unwind,
    },
    sync::Mutex,
};

use qubit_progress::{
    Metric,
    Progress,
    Reporter,
    TextReporter,
};

/// Verifies that the text sink emits one complete event record per delivery.
#[test]
fn test_text_reporter_writes_one_complete_line_per_event() {
    let reporter = TextReporter::new(Vec::new());
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("progress must start");
    drop(progress);
    let bytes = reporter
        .into_inner()
        .expect("text reporter writer mutex must not be poisoned");
    let output = String::from_utf8(bytes).expect("text output must be UTF-8");
    assert!(output.contains("phase=started"));
    assert!(output.contains("total=Some(1)"));
}

/// Verifies that the optional JSON Lines sink preserves one complete event.
#[cfg(feature = "json-lines")]
#[test]
fn test_json_lines_reporter_serializes_complete_event() {
    use qubit_progress::JsonLinesReporter;

    let reporter = JsonLinesReporter::new(Vec::new());
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("progress must start");
    drop(progress);
    let bytes = reporter
        .into_inner()
        .expect("JSON Lines reporter writer mutex must not be poisoned");
    let output = String::from_utf8(bytes).expect("JSON output must be UTF-8");
    assert!(output.contains("\"phase\":\"started\""));
    assert!(output.contains("\"total\":1"));
}

/// Verifies that the log reporter samples the facade's info-level enablement.
#[cfg(feature = "log")]
#[test]
fn test_log_reporter_matches_info_level_enablement() {
    use qubit_progress::LogReporter;

    let reporter = LogReporter;
    assert_eq!(
        reporter.is_enabled(),
        log::log_enabled!(log::Level::Info),
        "log reporting must not start operations when info output is disabled",
    );
}

/// Writer that rejects all writes to exercise reporter error propagation.
struct FailingWriter;

impl Write for FailingWriter {
    /// Rejects every write with a stable I/O error.
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("writer unavailable"))
    }

    /// Treats flushing as successful because no data is buffered.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Verifies TextReporter surfaces writer failures without losing the event
/// path.
#[test]
fn test_text_reporter_propagates_writer_failure() {
    let reporter = TextReporter::new(FailingWriter);
    let result = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start();
    let error = match result {
        Ok(_) => panic!("Started delivery must fail"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("writer unavailable"));
}

/// Verifies the reporter closure implementation accepts complete events.
#[test]
fn test_reporter_closure_receives_started_event() {
    let reporter = |event: &qubit_progress::Event| {
        assert_eq!(event.phase().as_str(), "started");
        Ok(())
    };
    let progress = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("closure reporter must accept Started");
    drop(progress);
}

/// Verifies the optional JSON Lines reporter surfaces writer failures.
#[cfg(feature = "json-lines")]
#[test]
fn test_json_lines_reporter_propagates_writer_failure() {
    use qubit_progress::JsonLinesReporter;

    let reporter = JsonLinesReporter::new(FailingWriter);
    let result = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start();
    let error = match result {
        Ok(_) => panic!("Started delivery must fail"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("writer unavailable"));
}

/// Writer that accepts an event record and rejects the trailing delimiter.
struct NewlineFailingWriter {
    /// Number of writes accepted before returning an error.
    writes: usize,
}

/// Writer that panics while a reporter holds its mutex.
struct PanickingWriter;

impl Write for PanickingWriter {
    /// Panics to poison the enclosing reporter mutex.
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        panic!("writer panicked")
    }

    /// Is unreachable because writing always panics.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Builds an event suitable for direct reporter delivery checks.
fn create_started_event() -> qubit_progress::Event {
    let events = Mutex::new(Vec::new());
    {
        let capture = |event: &qubit_progress::Event| {
            events
                .lock()
                .expect("event collection mutex must not be poisoned")
                .push(event.clone());
            Ok(())
        };
        let progress = Progress::builder(&capture)
            .metric(Metric::new("tasks", "Tasks"))
            .start()
            .expect("capture must accept the Started event");
        drop(progress);
    }
    events
        .into_inner()
        .expect("event collection mutex must not be poisoned")
        .pop()
        .expect("Started event must be captured")
}

/// Verifies each generic line reporter implementation is directly executable.
#[test]
fn test_line_reporters_report_every_writer_shape() {
    let event = create_started_event();
    TextReporter::new(Vec::new())
        .report(&event)
        .expect("text Vec writer must accept an event");
    assert!(TextReporter::new(FailingWriter).report(&event).is_err());
    assert!(
        TextReporter::new(NewlineFailingWriter { writes: 0 })
            .report(&event)
            .is_err()
    );
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = TextReporter::new(PanickingWriter).report(&event);
        }))
        .is_err()
    );

    #[cfg(feature = "json-lines")]
    {
        use qubit_progress::JsonLinesReporter;

        JsonLinesReporter::new(Vec::new())
            .report(&event)
            .expect("JSON Lines Vec writer must accept an event");
        assert!(
            JsonLinesReporter::new(FailingWriter)
                .report(&event)
                .is_err()
        );
        assert!(
            JsonLinesReporter::new(NewlineFailingWriter { writes: 0 })
                .report(&event)
                .is_err()
        );
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = JsonLinesReporter::new(PanickingWriter).report(&event);
            }))
            .is_err()
        );
    }
}

/// Verifies TextReporter returns its writer when a prior delivery panicked.
#[test]
fn test_text_reporter_into_inner_exposes_poisoned_writer() {
    let reporter = TextReporter::new(PanickingWriter);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = Progress::builder(&reporter)
            .metric(Metric::new("tasks", "Tasks"))
            .start();
    }));
    assert!(result.is_err());
    assert!(reporter.report(&create_started_event()).is_err());
    assert!(reporter.into_inner().is_err());
}

/// Verifies JSON Lines returns its writer when a prior delivery panicked.
#[cfg(feature = "json-lines")]
#[test]
fn test_json_lines_reporter_into_inner_exposes_poisoned_writer() {
    use qubit_progress::JsonLinesReporter;

    let reporter = JsonLinesReporter::new(PanickingWriter);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = Progress::builder(&reporter)
            .metric(Metric::new("tasks", "Tasks"))
            .start();
    }));
    assert!(result.is_err());
    assert!(reporter.report(&create_started_event()).is_err());
    assert!(reporter.into_inner().is_err());
}

impl Write for NewlineFailingWriter {
    /// Rejects the newline write after accepting the serialized event bytes.
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        if self.writes == 1 {
            Ok(buffer.len())
        } else {
            Err(io::Error::other("newline unavailable"))
        }
    }

    /// Treats flushing as successful because no data is buffered.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Verifies both line reporters surface a trailing-newline write failure.
#[test]
fn test_text_reporter_propagates_newline_failure() {
    let reporter = TextReporter::new(NewlineFailingWriter { writes: 0 });
    let result = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start();
    let error = match result {
        Ok(_) => panic!("newline delivery must fail"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("newline unavailable"));
}

/// Verifies JSON Lines also reports a trailing-newline write failure.
#[cfg(feature = "json-lines")]
#[test]
fn test_json_lines_reporter_propagates_newline_failure() {
    use qubit_progress::JsonLinesReporter;

    let reporter = JsonLinesReporter::new(NewlineFailingWriter { writes: 0 });
    let result = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start();
    let error = match result {
        Ok(_) => panic!("newline delivery must fail"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("newline unavailable"));
}

/// Verifies NoopReporter accepts direct reporter calls.
#[test]
fn test_noop_reporter_accepts_direct_delivery() {
    let reporter = |event: &qubit_progress::Event| {
        qubit_progress::NoopReporter.report(event)
    };
    let _ = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("NoopReporter must accept direct events");
}

/// Verifies LogReporter accepts direct reporter calls.
#[cfg(feature = "log")]
#[test]
fn test_log_reporter_accepts_direct_delivery() {
    use qubit_progress::LogReporter;

    let reporter = |event: &qubit_progress::Event| LogReporter.report(event);
    let _ = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("LogReporter must accept direct events");
}
