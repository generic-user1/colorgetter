//! Definition of the Bottle trait

use crate::bottle::{PourInError, PourOutError};
use crate::colored_water::{
    ColoredWaterRun, ColoredWaterUnit, PartialColoredWaterRun, PartialColoredWaterUnit
};

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

    /// Return whether this bottle has any content (i.e. there is some index for which [Bottle::sample_content_at] returns [Some]).
    ///
    /// A return value of `true` indicates there is some content, and a return value of `false` indicates there is no content.
    ///
    /// **Note**: The provided default implementation of this is iterative; it will call
    /// [Bottle::sample_content_at] repeatedly, up to [Bottle::capacity] times in the worst case.
    /// You should consider implementing this manually for better performance.
    fn is_empty(&self) -> bool {
        for idx in 0..self.capacity() {
            if self.sample_content_at(idx).is_some() {
                return false;
            }
        }
        true
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

    /// Estimate how many pours it would take to finish this bottle.
    ///
    /// This is done under a few assumptions:
    /// - Every pour out removes the entire top color run and is always possible
    /// - Every pour in adds exactly one unit of the desired color
    /// - The known color currently at the bottom (if there is one) is the color we want for this bottle
    /// - No two unknown colors are ever the same
    fn pours_to_finish_estimate(&self) -> usize {
        //the number of units at the bottom whose color matches
        let mut already_done_count = 0_usize;
        match self.sample_at(0) {
            BottleSampleResult::KnownColor(c) => {
                //check how many contiguous units match this color
                for idx in 0..self.capacity() {
                    let sample_result = self.sample_at(idx);
                    if sample_result == BottleSampleResult::KnownColor(c) {
                        already_done_count += 1;
                    } else {
                        break;
                    }
                }
            }
            BottleSampleResult::UnknownColor => {
                //we assert that this color isn't the one we want to end on, and
                //that it doesn't match the next color up
                already_done_count = 0;
            }
            BottleSampleResult::Empty => {
                //do nothing; leave already_done_count at 0
            }
            BottleSampleResult::OutOfBounds => {
                //bottle has zero capacity so it must be finished
                return 0;
            }
        }

        //we now need to detect how many pours it would take to remove all units of non-matching color.
        //put another way, we need to detect how many color runs there are in the bottle, not including our bottom run
        let mut run_count = 0_usize;
        let mut current_run_color: Option<PartialColoredWaterUnit> = None;
        for idx in already_done_count..self.capacity() {
            match self.sample_content_at(idx) {
                Some(PartialColoredWaterUnit::Color(c)) => {
                    if current_run_color.is_none()
                        || current_run_color.unwrap() != PartialColoredWaterUnit::Color(c)
                    {
                        run_count += 1;
                        current_run_color = Some(PartialColoredWaterUnit::Color(c))
                    }
                }
                Some(PartialColoredWaterUnit::UnknownColor) => {
                    //unknown colors are assumed to never match, even other unknowns
                    run_count += 1;
                    current_run_color = Some(PartialColoredWaterUnit::UnknownColor);
                }
                None => {
                    //if we hit empty space, we know we're done counting runs
                    break;
                }
            }
        }
        //the number of pours needed is the run_count we just calculated plus
        //the number of units we need to pour in (capacity - already_done_count)
        run_count + (self.capacity() - already_done_count)
    }

    /// Estimate how close to being "finished" this bottle is as a value in the range `[0.0, 1.0]`
    ///
    /// `1.0` means entirely finished, and `0.0` means entirely unfinished. Note that it is valid
    /// for the absolute minimum value to be greater than `0.0` and for the absolute maximum value to be less than `1.0`,
    /// though it is not valid for the output to be outside of these bounds.
    fn finished_estimate(&self) -> f64 {
        //first, estimate the number of pours required to finish this bottle
        let pours_needed = self.pours_to_finish_estimate();

        //transform this number of pours needed into a score
        //to do that, we'll first turn it into a proportion of the maximum number of
        //pours to finish a bottle of this size (always `capacity * 2`)
        let as_proportion = (pours_needed as f64) / ((self.capacity() * 2) as f64);

        //finally, we invert the proportion by subtracting it from 1
        1.0 - as_proportion
    }
    /// Attempt to pour a [ColoredWaterRun] into this bottle.
    ///
    /// If this is successful, will return a new [ColoredWaterRun] representing the portion of
    /// the given `content_to_pour` that wouldn't fit into this bottle. The `size` of this returned [ColoredWaterRun]
    /// may be 0; this indicates that the entirity of `content_to_pour` fit into this bottle.
    ///
    /// If this is unsuccessful, an [Err] is returned with an appropriate [PourInError] variant.
    /// No change is made to this bottle in this case.
    ///
    /// Note that this only works with [ColoredWaterRun], not [PartialColoredWaterRun] - unknown colors cannot be poured,
    /// because it's normally not possible to know whether their color matches the destination's.
    fn try_pour_in(
        &mut self,
        content_to_pour: ColoredWaterRun
    ) -> Result<ColoredWaterRun, PourInError>;

    /// Determine if running [Bottle::try_pour_in] would succeed given the current content of this bottle
    /// and the provided `content_to_pour`, but don't actually modify the content of this bottle.
    ///
    /// Return value is the same as the return value of [Bottle::try_pour_in] would be if called on this same
    /// bottle with the same `content_to_pour`
    fn test_pour_in(
        &self,
        content_to_pour: ColoredWaterRun
    ) -> Result<ColoredWaterRun, PourInError>;

    /// Attempt to pour a [ColoredWaterRun] out of this bottle.
    ///
    /// If this is successful, will return `Ok(())`.
    ///
    /// If this is unsuccessful, an [Err] is returned with an appropriate [PourOutError] variant.
    /// No change is made to either this bottle or the destination bottle in this case.
    fn try_pour_out<T: Bottle>(&mut self, destination: &mut T) -> Result<(), PourOutError>;

    /// Determine if running [Bottle::try_pour_out] would succeed given the current content of this bottle
    /// and the provided `destination` bottle, but don't actually modify the content of either bottle.
    ///
    /// Return value is the same as the return value of [Bottle::try_pour_out] would be if called on this same
    /// bottle with the same `destination` bottle.
    fn test_pour_out<T: Bottle>(&self, destination: &T) -> Result<(), PourOutError>;
}
