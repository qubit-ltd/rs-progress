// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::io::{
    self,
    Write,
};

/// Writer that deterministically rejects every write.
pub(crate) struct FailingWriter;

impl Write for FailingWriter {
    /// Rejects every byte slice with a synthetic I/O error.
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("synthetic progress output failure"))
    }

    /// Rejects every flush with a synthetic I/O error.
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("synthetic progress output failure"))
    }
}
