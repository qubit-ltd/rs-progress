// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Reporter output and event serialization tests.

use std::io;
use std::io::BufWriter;
use std::io::Write;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::Mutex;

use qubit_progress::Event;
use qubit_progress::Metric;
use qubit_progress::NoopReporter;
use qubit_progress::Progress;
use qubit_progress::Reporter;
use qubit_progress::TextReporter;

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
    let reporter = |event: &Event| {
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

/// Writer that records each complete write request.
#[derive(Default)]
struct WriteCallRecorder {
    /// Buffers supplied to `write` in call order.
    writes: Vec<Vec<u8>>,
}

/// Writer that accepts at most a few bytes per call and records flushes.
#[derive(Default)]
struct ShortWriter {
    bytes: Vec<u8>,
    flushes: usize,
    fail_flush: bool,
}

impl Write for ShortWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let count = buffer.len().min(3);
        self.bytes.extend_from_slice(&buffer[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        if self.fail_flush {
            Err(io::Error::other("flush unavailable"))
        } else {
            Ok(())
        }
    }
}

/// Writes a fixed prefix, then rejects subsequent writes.
#[derive(Default)]
struct PrefixThenFailWriter {
    bytes: Vec<u8>,
    fail_next_write: bool,
    prefix_written: bool,
}

impl Write for PrefixThenFailWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.prefix_written && self.fail_next_write {
            self.fail_next_write = false;
            return Err(io::Error::other("write failed after prefix"));
        }
        if !self.prefix_written {
            let count = buffer.len().min(5);
            self.bytes.extend_from_slice(&buffer[..count]);
            self.prefix_written = true;
            self.fail_next_write = true;
            Ok(count)
        } else {
            self.bytes.extend_from_slice(buffer);
            Ok(buffer.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
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
fn create_started_event() -> Event {
    let events = Mutex::new(Vec::new());
    {
        let capture = |event: &Event| {
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

/// Captures Started, explicitly reported Running, and Cancelled events.
fn create_cancelled_lifecycle_events() -> Vec<Event> {
    let events = Mutex::new(Vec::new());
    {
        let capture = |event: &Event| {
            events
                .lock()
                .expect("event collection mutex must not be poisoned")
                .push(event.clone());
            Ok(())
        };
        let mut progress = Progress::builder(&capture)
            .metric(Metric::new("tasks", "Tasks").total(1))
            .start()
            .expect("capture must accept the Started event");
        let tasks = progress.metric("tasks").expect("configured metric must exist");
        tasks.start(1).expect("work must start");
        progress.report().expect("capture must accept the Running event");
        progress.cancel().expect("capture must accept the Cancelled event");
    }
    events
        .into_inner()
        .expect("event collection mutex must not be poisoned")
}

/// Reports Started, Running, and Cancelled through a public Progress handle.
fn report_cancelled_lifecycle(reporter: &impl Reporter) {
    let mut progress = Progress::builder(reporter)
        .metric(Metric::new("tasks", "Tasks").total(1))
        .start()
        .expect("reporter must accept the Started event");
    let tasks = progress.metric("tasks").expect("configured metric must exist");
    tasks.start(1).expect("work must start");
    progress.report().expect("reporter must accept the Running event");
    progress.cancel().expect("reporter must accept the Cancelled event");
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
        assert!(JsonLinesReporter::new(FailingWriter).report(&event).is_err());
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

impl Write for WriteCallRecorder {
    /// Records the complete buffer passed to one write call.
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.writes.push(buffer.to_vec());
        Ok(buffer.len())
    }

    /// Treats flushing as successful because no data is buffered.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Verifies TextReporter writes one complete newline-delimited record at once.
#[test]
fn test_text_reporter_writes_complete_line_in_one_write() {
    let event = create_started_event();
    let reporter = TextReporter::new(WriteCallRecorder::default());
    reporter
        .report(&event)
        .expect("recording writer must accept the text event");
    let writer = reporter.into_inner().expect("text reporter mutex must not be poisoned");
    assert_eq!(writer.writes.len(), 1);
    assert!(writer.writes[0].ends_with(b"\n"));
}

/// Verifies short writes are completed and flush remains caller-controlled.
#[test]
fn test_text_reporter_short_writes_and_explicit_flush() {
    let reporter = TextReporter::new(ShortWriter::default());
    report_cancelled_lifecycle(&reporter);
    let mut writer = reporter.into_inner().expect("writer mutex must be healthy");
    let output = String::from_utf8(writer.bytes.clone()).expect("text records must be UTF-8");
    let lines: Vec<_> = output.lines().collect();
    assert_eq!(lines.len(), 3, "each lifecycle event must remain a line");
    assert!(lines[0].contains("phase=started"));
    assert!(lines[1].contains("phase=running"));
    assert!(lines[2].contains("phase=cancelled"));
    assert_eq!(writer.flushes, 0, "report and into_inner must not flush");
    writer.flush().expect("explicit flush must succeed");
    assert_eq!(writer.flushes, 1);

    let mut writer = BufWriter::new(ShortWriter {
        fail_flush: true,
        ..ShortWriter::default()
    });
    writer.write_all(b"buffered").expect("buffered write must succeed");
    assert!(
        writer
            .flush()
            .expect_err("flush error must be observable")
            .to_string()
            .contains("flush unavailable")
    );
}

/// Verifies a partial write failure is visible and later calls do not repair
/// it.
#[test]
fn test_text_reporter_partial_write_failure_does_not_repair_boundary() {
    let mut events = create_cancelled_lifecycle_events().into_iter();
    let started = events.next().expect("Started event must exist");
    let cancelled = events.last().expect("Cancelled event must exist");
    let reporter = TextReporter::new(PrefixThenFailWriter::default());
    assert!(reporter.report(&started).is_err());
    reporter
        .report(&cancelled)
        .expect("one-shot writer failure must recover");
    let writer = reporter.into_inner().expect("writer mutex must be healthy");
    assert!(writer.bytes.starts_with(b"opera"), "failed write leaves its prefix");
    assert!(
        writer
            .bytes
            .windows(b"phase=cancelled".len())
            .any(|window| window == b"phase=cancelled")
    );
    assert!(
        writer.bytes[5..].starts_with(b"operation="),
        "Cancelled record follows the prefix immediately"
    );
    assert!(
        !writer.bytes[5..].starts_with(b"\n"),
        "later record is not prefixed with a repaired newline"
    );
}

/// Verifies into_inner does not flush buffered records and flush errors
/// surface.
#[test]
fn test_text_reporter_bufwriter_flush_contract() {
    let reporter = TextReporter::new(BufWriter::with_capacity(4096, Vec::new()));
    report_cancelled_lifecycle(&reporter);
    let mut writer = reporter.into_inner().expect("writer mutex must be healthy");
    assert!(writer.get_ref().is_empty(), "into_inner must not flush");
    writer.flush().expect("explicit flush must succeed");
    let bytes = writer.into_inner().expect("buffered writer must not fail");
    assert_eq!(bytes.iter().filter(|byte| **byte == b'\n').count(), 3);
}

/// Verifies JSON Lines writes one complete newline-delimited record at once.
#[cfg(feature = "json-lines")]
#[test]
fn test_json_lines_reporter_writes_complete_line_in_one_write() {
    use qubit_progress::JsonLinesReporter;

    let event = create_started_event();
    let reporter = JsonLinesReporter::new(WriteCallRecorder::default());
    reporter
        .report(&event)
        .expect("recording writer must accept the JSON event");
    let writer = reporter
        .into_inner()
        .expect("JSON Lines reporter mutex must not be poisoned");
    assert_eq!(writer.writes.len(), 1);
    assert!(writer.writes[0].ends_with(b"\n"));
}

/// Verifies JSON Lines short writes complete a record without implicit flush.
#[cfg(feature = "json-lines")]
#[test]
fn test_json_lines_reporter_short_writes_and_explicit_flush() {
    use qubit_progress::JsonLinesReporter;

    let reporter = JsonLinesReporter::new(ShortWriter::default());
    report_cancelled_lifecycle(&reporter);
    let mut writer = reporter.into_inner().expect("writer mutex must be healthy");
    let output = String::from_utf8(writer.bytes.clone()).expect("JSON Lines output must be UTF-8");
    let lines: Vec<_> = output.lines().collect();
    assert_eq!(lines.len(), 3, "each lifecycle event must remain a line");
    let phases: Vec<_> = lines
        .iter()
        .map(|line| {
            serde_json::from_str::<Event>(line)
                .expect("each line must be a complete event")
                .phase()
                .as_str()
                .to_owned()
        })
        .collect();
    assert_eq!(phases, ["started", "running", "cancelled"]);
    assert_eq!(writer.flushes, 0, "report and into_inner must not flush");
    writer.flush().expect("explicit flush must succeed");
    assert_eq!(writer.flushes, 1);

    let mut writer = BufWriter::new(ShortWriter {
        fail_flush: true,
        ..ShortWriter::default()
    });
    writer.write_all(b"buffered").expect("buffered write must succeed");
    assert!(
        writer
            .flush()
            .expect_err("flush error must be observable")
            .to_string()
            .contains("flush unavailable")
    );
}

/// Verifies a JSON Lines partial write failure does not repair later
/// boundaries.
#[cfg(feature = "json-lines")]
#[test]
fn test_json_lines_reporter_partial_write_failure_does_not_repair_boundary() {
    use qubit_progress::JsonLinesReporter;

    let mut events = create_cancelled_lifecycle_events().into_iter();
    let started = events.next().expect("Started event must exist");
    let cancelled = events.last().expect("Cancelled event must exist");
    let reporter = JsonLinesReporter::new(PrefixThenFailWriter::default());
    assert!(reporter.report(&started).is_err());
    reporter
        .report(&cancelled)
        .expect("one-shot writer failure must recover");
    let writer = reporter.into_inner().expect("writer mutex must be healthy");
    assert!(writer.bytes.starts_with(b"{"), "failed write leaves its prefix");
    assert!(
        writer
            .bytes
            .windows(b"\"phase\":\"cancelled\"".len())
            .any(|window| window == b"\"phase\":\"cancelled\"")
    );
    assert!(
        writer.bytes[5..].starts_with(b"{"),
        "Cancelled record follows the prefix immediately"
    );
    assert!(
        !writer.bytes[5..].starts_with(b"\n"),
        "later record is not prefixed with a repaired newline"
    );
}

/// Verifies JSON Lines into_inner leaves buffered bytes for explicit flush.
#[cfg(feature = "json-lines")]
#[test]
fn test_json_lines_reporter_bufwriter_flush_contract() {
    use qubit_progress::JsonLinesReporter;

    let reporter = JsonLinesReporter::new(BufWriter::with_capacity(4096, Vec::new()));
    report_cancelled_lifecycle(&reporter);
    let mut writer = reporter.into_inner().expect("writer mutex must be healthy");
    assert!(writer.get_ref().is_empty(), "into_inner must not flush");
    writer.flush().expect("explicit flush must succeed");
    let bytes = writer.into_inner().expect("buffered writer must not fail");
    assert_eq!(bytes.iter().filter(|byte| **byte == b'\n').count(), 3);
}

/// Verifies NoopReporter accepts direct reporter calls.
#[test]
fn test_noop_reporter_accepts_direct_delivery() {
    let reporter = |event: &Event| NoopReporter.report(event);
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

    let reporter = |event: &Event| LogReporter.report(event);
    let _ = Progress::builder(&reporter)
        .metric(Metric::new("tasks", "Tasks"))
        .start()
        .expect("LogReporter must accept direct events");
}
