//! Definition of the Bottle trait

use crate::colored_water::{ColoredWaterUnit, PartialColoredWaterRun, PartialColoredWaterUnit};

///Reasons that setting a [PartialColoredWaterUnit](crate::colored_water::PartialColoredWaterUnit) within a [Bottle] may fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSetError {
    /// Attempted to set a unit to a color (known or unknown) at a location that has empty space below it
    EmptyBelow,

    /// Attempted to set a unit to empty at a location that has non-empty space above it
    FullAbove,

    /// Attempted to set a unit to an unknown color at a location that has known colors below it
    KnownBelow,

    /// Attempted to set a unit to a known color at a location that has unknown colors above it
    UnknownAbove,

    /// Attempted to set a unit at a location beyond the capacity of the destination bottle
    ExceedsCapacity,

    /// Attempted to set a unit to [PartialColoredWaterUnit::UnknownColor](crate::colored_water::PartialColoredWaterUnit::UnknownColor)
    /// in a bottle that only supports [PartialColoredWaterUnit::Color](crate::colored_water::PartialColoredWaterUnit::Color)
    UnknownNotSupported
}

/// The possible values that [Bottle::sample_at] may return
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

impl From<ColoredWaterUnit> for BottleSampleResult {
    fn from(value: ColoredWaterUnit) -> Self {
        BottleSampleResult::KnownColor(value)
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

/// Types representing bottles of colored water.
///
/// This abstracts some of the behavior for [KnownBottle](super::KnownBottle) and [PartialBottle](super::PartialBottle)
pub trait Bottle {
    /// Try to set the [PartialColoredWaterUnit] at index `idx` within this bottle to the given `new_color`
    ///
    /// If `new_color` is [Some] and the inner value is [PartialColoredWaterUnit::Color], will try to set the
    /// specified unit to the specified [ColoredWaterUnit].
    ///
    /// If `new_color` is [Some] and the inner value is [PartialColoredWaterUnit::UnknownColor], will try to set the
    /// specified unit to an unknown unit.
    ///
    /// If `new_color` is [None], will instead try to clear the unit at the given `idx` so that it becomes empty.
    ///
    /// If this fails (i.e. returns [Err]), the Bottle will be left unchanged.
    fn try_set_color(
        &mut self,
        idx: usize,
        new_color: Option<PartialColoredWaterUnit>
    ) -> Result<(), ColorSetError>;

    /// Sample the color in this bottle at the given index.
    fn sample_at(&self, idx: usize) -> BottleSampleResult;

    /// Return the capacity of this bottle
    fn capacity(&self) -> usize;

    /// Returns the [PartialColoredWaterUnit] at the top of this bottle
    ///
    /// This returns [None] if there isn't any water in the bottle.
    fn get_top_color(&self) -> Option<PartialColoredWaterUnit> {
        if let Some(idx) = self.get_top_content_idx() {
            self.sample_content_at(idx)
        } else {
            None
        }
    }

    /// Returns the [PartialColoredWaterRun] at the top of this bottle
    ///
    /// This returns [None] if there isn't any water in the bottle.
    fn get_top_color_run(&self) -> Option<PartialColoredWaterRun> {
        if let Some(top_idx) = self.get_top_content_idx() {
            let color = self.sample_content_at(top_idx).unwrap();
            let mut color_count = 0;
            let mut cur_idx = top_idx;
            loop {
                let sample_result = self.sample_content_at(cur_idx).unwrap();
                if sample_result == color {
                    color_count += 1;
                    if let Some(new_idx) = cur_idx.checked_sub(1) {
                        cur_idx = new_idx;
                    } else {
                        //we've looped through all colors we can
                        break;
                    }
                } else {
                    //we found a color that didn't match
                    break;
                }
            }
            Some(PartialColoredWaterRun {
                color,
                size: color_count
            })
        } else {
            None
        }
    }

    /// Return the largest index in this bottle for which [Bottle::sample_content_at]
    /// returns [Some].
    ///
    /// A return value of [None] indicates that there is no content at all.
    ///
    /// **Note**: The provided default implementation of this is iterative; it will call
    /// [Bottle::sample_content_at] repeatedly, up to [Bottle::capacity] times in the worst case.
    /// You should consider implementing this manually for better performance.
    fn get_top_content_idx(&self) -> Option<usize> {
        for idx in (0..self.capacity()).rev() {
            let sample_result = self.sample_content_at(idx);
            if sample_result.is_some() {
                return Some(idx);
            }
        }
        None
    }

    /// Sample the color (known or unknown) in this bottle at the given index. If there is no color
    /// at that index for any reason, return [None].
    fn sample_content_at(&self, idx: usize) -> Option<PartialColoredWaterUnit> {
        self.sample_at(idx).try_into().ok()
    }

    /// Sample the known color in this bottle at the given index. If there is no color at
    /// that index for any reason, or if there is a color but it's unknown, return [None].
    fn sample_known_color_at(&self, idx: usize) -> Option<ColoredWaterUnit> {
        self.sample_at(idx).try_into().ok()
    }
}
