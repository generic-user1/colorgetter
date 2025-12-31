//! Definition of the BottleSample trait and implementations for [Bottle] and [PartialBottle]

use super::{Bottle, PartialBottle};
use crate::colored_water::{ColoredWaterUnit, PartialColoredWaterUnit};

/// The possible values that [BottleSample::sample_at] may return
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BottleSampleResult {
    /// There is a known color at the sampled location
    KnownColor(ColoredWaterUnit),

    /// There is an unknown color at the sampled location
    UnknownColor,

    /// There is empty space at the sampled location, but the location
    /// is still within the capacity of the sampled item
    Empty,

    /// The sampled location is outside the capacity of the sampled item
    OutOfBounds
}

impl From<PartialColoredWaterUnit> for BottleSampleResult {
    fn from(value: PartialColoredWaterUnit) -> Self {
        match value {
            PartialColoredWaterUnit::Color(c) => BottleSampleResult::KnownColor(c),
            PartialColoredWaterUnit::UnknownColor => BottleSampleResult::UnknownColor
        }
    }
}

/// Reasons converting a [BottleSampleResult] into a [ColoredWaterUnit] or [PartialColoredWaterUnit] may fail
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BottleSampleConversionError {
    /// Trying to convert into a [ColoredWaterUnit] but the specific color sampled isn't known
    UnknownColor,

    /// The sample did not result in any color - either because it was [BottleSampleResult::Empty] or
    /// [BottleSampleResult::OutOfBounds]
    NoColor
}

impl TryFrom<BottleSampleResult> for PartialColoredWaterUnit {
    type Error = BottleSampleConversionError;
    fn try_from(value: BottleSampleResult) -> Result<Self, Self::Error> {
        match value {
            BottleSampleResult::KnownColor(c) => Ok(PartialColoredWaterUnit::Color(c)),
            BottleSampleResult::UnknownColor => Ok(PartialColoredWaterUnit::UnknownColor),
            BottleSampleResult::Empty | BottleSampleResult::OutOfBounds => {
                Err(BottleSampleConversionError::NoColor)
            }
        }
    }
}

impl TryFrom<BottleSampleResult> for ColoredWaterUnit {
    type Error = BottleSampleConversionError;
    fn try_from(value: BottleSampleResult) -> Result<Self, Self::Error> {
        match value {
            BottleSampleResult::KnownColor(c) => Ok(c),
            BottleSampleResult::UnknownColor => Err(BottleSampleConversionError::UnknownColor),
            BottleSampleResult::Empty | BottleSampleResult::OutOfBounds => {
                Err(BottleSampleConversionError::NoColor)
            }
        }
    }
}

impl From<ColoredWaterUnit> for BottleSampleResult {
    fn from(value: ColoredWaterUnit) -> Self {
        BottleSampleResult::KnownColor(value)
    }
}

/// Types representing bottles that can be "sampled" for display purposes
///
/// This currently only includes [Bottle] and [PartialBottle]
pub trait BottleSample {
    /// Sample the color in this bottle at the given index.
    fn sample_at(&self, idx: usize) -> BottleSampleResult;

    /// Return the capacity of this bottle
    fn capacity(&self) -> usize;
}

impl<const MAX_CAP: usize> BottleSample for Bottle<MAX_CAP> {
    fn capacity(&self) -> usize {
        self.get_capacity()
    }

    fn sample_at(&self, idx: usize) -> BottleSampleResult {
        if idx > self.capacity() {
            BottleSampleResult::OutOfBounds
        } else {
            let sampled = self.get_content().get(idx);
            if let Some(&sampled) = sampled {
                sampled.into()
            } else {
                BottleSampleResult::Empty
            }
        }
    }
}

impl<const MAX_CAP: usize> BottleSample for PartialBottle<MAX_CAP> {
    fn capacity(&self) -> usize {
        self.get_capacity()
    }

    fn sample_at(&self, idx: usize) -> BottleSampleResult {
        if idx > self.capacity() {
            BottleSampleResult::OutOfBounds
        } else if idx > self.get_unknown_count() {
            // adjust idx so that idx 0 points to first known color
            let idx = idx - self.get_unknown_count();
            let sampled = self.get_content().get(idx);
            if let Some(&sampled) = sampled {
                sampled.into()
            } else {
                BottleSampleResult::Empty
            }
        } else {
            // idx is inside our unknown count so it must point to unknown
            BottleSampleResult::UnknownColor
        }
    }
}
