/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
/// Describes the current stage of a multi-stage operation.
///
/// # Examples
///
/// ```
/// use qubit_progress::ProgressStage;
///
/// let stage = ProgressStage::new("verify", "Verify files")
///     .with_index(2)
///     .with_total_stages(4)
///     .with_weight(0.25);
///
/// assert_eq!(stage.id(), "verify");
/// assert_eq!(stage.name(), "Verify files");
/// assert_eq!(stage.index(), Some(2));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressStage {
    /// Stable machine-readable stage identifier.
    id: String,
    /// Human-readable stage name.
    name: String,
    /// Zero-based stage index when known.
    index: Option<usize>,
    /// Total number of stages when known.
    total_stages: Option<usize>,
    /// Relative stage weight when the caller uses weighted progress.
    weight: Option<f64>,
}

impl ProgressStage {
    /// Creates a stage with a stable id and display name.
    ///
    /// # Parameters
    ///
    /// * `id` - Stable machine-readable identifier.
    /// * `name` - Human-readable stage name.
    ///
    /// # Returns
    ///
    /// A stage with no index, total stage count, or weight.
    #[inline]
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            index: None,
            total_stages: None,
            weight: None,
        }
    }

    /// Returns a copy configured with a zero-based stage index.
    ///
    /// # Parameters
    ///
    /// * `index` - Zero-based stage index.
    ///
    /// # Returns
    ///
    /// This stage with `index` recorded.
    #[inline]
    pub const fn with_index(mut self, index: usize) -> Self {
        self.index = Some(index);
        self
    }

    /// Returns a copy configured with the total stage count.
    ///
    /// # Parameters
    ///
    /// * `total_stages` - Total number of stages in the operation.
    ///
    /// # Returns
    ///
    /// This stage with `total_stages` recorded.
    #[inline]
    pub const fn with_total_stages(mut self, total_stages: usize) -> Self {
        self.total_stages = Some(total_stages);
        self
    }

    /// Returns a copy configured with a relative stage weight.
    ///
    /// The weight is intended for caller-side weighted progress calculations.
    /// Callers should supply finite, non-negative values. This method records
    /// the supplied value as-is and does not validate `NaN`, infinity, or
    /// negative input.
    ///
    /// # Parameters
    ///
    /// * `weight` - Finite, non-negative relative stage weight used by callers
    ///   that compute weighted total progress.
    ///
    /// # Returns
    ///
    /// This stage with `weight` recorded.
    #[inline]
    pub const fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Returns the stable stage identifier.
    ///
    /// # Returns
    ///
    /// The machine-readable stage id.
    #[inline]
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    /// Returns the human-readable stage name.
    ///
    /// # Returns
    ///
    /// The display name for this stage.
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the stage index when known.
    ///
    /// # Returns
    ///
    /// `Some(index)` when a zero-based stage index was supplied, otherwise
    /// `None`.
    #[inline]
    pub const fn index(&self) -> Option<usize> {
        self.index
    }

    /// Returns the total stage count when known.
    ///
    /// # Returns
    ///
    /// `Some(total)` when a total stage count was supplied, otherwise `None`.
    #[inline]
    pub const fn total_stages(&self) -> Option<usize> {
        self.total_stages
    }

    /// Returns the relative stage weight when known.
    ///
    /// # Returns
    ///
    /// `Some(weight)` when a weight was supplied, otherwise `None`.
    #[inline]
    pub const fn weight(&self) -> Option<f64> {
        self.weight
    }
}
