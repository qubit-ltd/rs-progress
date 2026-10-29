// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Immutable-friendly operation correlation attributes.

use std::collections::BTreeMap;
use std::sync::Arc;

/// String key-value attributes shared by every event in one operation.
///
/// # Examples
///
/// ```
/// use qubit_progress::OperationAttributes;
///
/// let mut attributes = OperationAttributes::new();
/// attributes.insert("trace_id", "trace-123");
/// assert_eq!(attributes.get("trace_id"), Some("trace-123"));
/// ```
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationAttributes {
    /// Stable ordered attribute entries.
    entries: BTreeMap<Arc<str>, Arc<str>>,
}

impl OperationAttributes {
    /// Creates an empty attribute set.
    ///
    /// # Returns
    ///
    /// An attribute set with no entries.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns one attribute value by key.
    ///
    /// # Parameters
    ///
    /// - `key`: The attribute key to look up.
    ///
    /// # Returns
    ///
    /// `Some(value)` when the key exists, or `None` when it does not.
    #[inline]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(AsRef::as_ref)
    }

    /// Returns all attributes in stable key order.
    ///
    /// # Returns
    ///
    /// A lazy iterator over key-value pairs in ascending key order.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.entries.iter().map(|(key, value)| (key.as_ref(), value.as_ref()))
    }

    /// Returns whether no attributes are configured.
    ///
    /// # Returns
    ///
    /// `true` when the set is empty, otherwise `false`.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Inserts or replaces one attribute value.
    ///
    /// # Parameters
    ///
    /// - `key`: The key whose value should be inserted or replaced.
    /// - `value`: The value to associate with `key`.
    pub fn insert(&mut self, key: &str, value: &str) {
        self.entries.insert(Arc::from(key), Arc::from(value));
    }
}
