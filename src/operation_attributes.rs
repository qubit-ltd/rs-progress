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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationAttributes {
    /// Stable ordered attribute entries.
    entries: BTreeMap<Arc<str>, Arc<str>>,
}

impl OperationAttributes {
    /// Creates an empty attribute set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces one attribute value.
    pub fn insert(&mut self, key: &str, value: &str) {
        self.entries.insert(Arc::from(key), Arc::from(value));
    }

    /// Returns one attribute value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(AsRef::as_ref)
    }

    /// Returns all attributes in stable key order.
    #[must_use = "the iterator yields configured attributes"]
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.entries
            .iter()
            .map(|(key, value)| (key.as_ref(), value.as_ref()))
    }

    /// Returns whether no attributes are configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
