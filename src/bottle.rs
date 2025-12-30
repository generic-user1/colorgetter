//! Implementation of a bottle for colored water

use crate::colored_water::{ColoredWaterRun, ColoredWaterUnit};
use heapless::Vec;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::{Debug, Display}
};

#[cfg(test)]
mod bottle_tests;

mod partial_bottle;
pub use partial_bottle::{PartialBottle, PartialBottleConversionError, PartialColorSetError};

/// One Bottle that may contain [ColoredWaterUnit]s
///
/// Each bottle has a capacity, and some content. The capacity is the number of water units the bottle is allowed to hold,
/// and the content is the different water units that the bottle is currently holding.
///
/// Note that the content may be shorter than the capacity (meaning the bottle has space for more content), but
/// will never be longer.
///
/// For performance reasons, Bottles must not require heap allocations. To this end, each bottle has a `MAX_CAP`; this is
/// the maximum capacity that particular bottle can have. Their actual capacity can be any value at or below `MAX_CAP`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(try_from = "UncheckedBottle<MAX_CAP>")]
pub struct Bottle<const MAX_CAP: usize> {
    capacity: usize,
    content: Vec<ColoredWaterUnit, MAX_CAP>
}

/// A Bottle that is directly deserializable but has no guarantee that
/// the capacity matches the content. Can be converted into a normal [Bottle]
/// with the [TryFrom]/[TryInto] traits.
#[derive(Deserialize)]
struct UncheckedBottle<const MAX_CAP: usize> {
    capacity: usize,
    content: Vec<ColoredWaterUnit, MAX_CAP>
}
impl<const MAX_CAP: usize> TryFrom<UncheckedBottle<MAX_CAP>> for Bottle<MAX_CAP> {
    type Error = BottleCapacityError;
    fn try_from(value: UncheckedBottle<MAX_CAP>) -> Result<Self, Self::Error> {
        if value.content.len() <= value.capacity {
            Ok(Bottle {
                capacity: value.capacity,
                content: value.content
            })
        } else {
            Err(BottleCapacityError::CapExceeded)
        }
    }
}

/// All reasons why creating or resizing a [Bottle] or [PartialBottle] may fail
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BottleCapacityError {
    /// The capacity requested is greater than the `MAX_CAP` of the Bottle
    MaxCapExceeded,

    /// The bottle is required to have more content than it has capacity. For [Bottle],
    /// this can only occur when deserializing. For [PartialBottle], this may occur during
    /// deserialization, but can also occur due to invalid combinations of `capacity` and `unknown_count`.
    CapExceeded
}
impl Display for BottleCapacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BottleCapacityError::MaxCapExceeded => write!(f, "Bottle Maximum Capacity Exceeded"),
            BottleCapacityError::CapExceeded => write!(f, "Bottle Capacity Exceeded")
        }
    }
}

/// Subset of [BottleCapacityError] including only errors that can occur
/// during normal use of [Bottle] (i.e. excluding deserialization and [PartialBottle])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BottleMaxCapError {
    /// The capacity requested is greater than the `MAX_CAP` of the Bottle
    MaxCapExceeded
}

impl From<BottleMaxCapError> for BottleCapacityError {
    fn from(value: BottleMaxCapError) -> Self {
        match value {
            BottleMaxCapError::MaxCapExceeded => BottleCapacityError::MaxCapExceeded
        }
    }
}

impl<const MAX_CAP: usize> Bottle<MAX_CAP> {
    /// Tries to create a new, empty Bottle
    ///
    /// This will fail if the given `capacity` is greater than `MAX_CAP`
    pub const fn try_new(capacity: usize) -> Result<Self, BottleMaxCapError> {
        if capacity <= MAX_CAP {
            Ok(Bottle {
                capacity,
                content: Vec::new()
            })
        } else {
            Err(BottleMaxCapError::MaxCapExceeded)
        }
    }

    /// Tries to create a new Bottle with the given content. Capacity is set to the number of elements in `content`.
    ///
    /// This will fail if the size of `content` is greater than `MAX_CAP`
    pub fn try_with_content(content: &[ColoredWaterUnit]) -> Result<Self, BottleMaxCapError> {
        Ok(Bottle {
            capacity: content.len(),
            content: Vec::from_slice(content).map_err(|_| BottleMaxCapError::MaxCapExceeded)?
        })
    }

    /// Immutably borrow the content of this Bottle.
    ///
    /// Note that the length of the return value may be less than the capacity of this Bottle,
    /// though it will never be greater.
    pub fn get_content(&self) -> &[ColoredWaterUnit] {
        &self.content
    }

    /// Try to set the [ColoredWaterUnit] at index `idx` within this bottle to the given `new_color`
    ///
    /// If `new_color` is [None], will instead try to clear the [ColoredWaterUnit] at the given `idx` so that it becomes empty.
    ///
    /// If this fails (i.e. returns [Err]), the Bottle will be left unchanged.
    pub fn try_set_color(
        &mut self,
        idx: usize,
        new_color: Option<ColoredWaterUnit>
    ) -> Result<(), ColorSetError> {
        if idx > self.content.len() {
            //trying to set color more than 1 above our current highest
            Err(ColorSetError::EmptyBelow)
        } else if idx == self.content.len() {
            //trying to set color one above our current highest
            if self.content.len() + 1 > self.capacity {
                Err(ColorSetError::ExceedsCapacity)
            } else {
                if let Some(new_color) = new_color {
                    self.content.push(new_color).unwrap();
                }
                Ok(())
            }
        } else if (idx + 1) == self.content.len() {
            //trying to set our current highest color
            if let Some(new_color) = new_color {
                *self.content.get_mut(idx).unwrap() = new_color;
            } else {
                self.content.pop();
            }
            Ok(())
        } else {
            //trying to set a color below our highest color
            if let Some(new_color) = new_color {
                *self.content.get_mut(idx).unwrap() = new_color;
                Ok(())
            } else {
                Err(ColorSetError::FullAbove)
            }
        }
    }

    /// Return the capacity of this Bottle.
    pub const fn get_capacity(&self) -> usize {
        self.capacity
    }

    /// Return the maximum capacity of this Bottle
    pub const fn get_max_capacity(&self) -> usize {
        MAX_CAP
    }

    /// Sets a new capacity for this Bottle.
    ///
    /// If `new_capacity` is larger than current capacity, empty space will be added to the 'top' of the Bottle.
    /// If `new_capacity` is smaller than current capacity, space (and any water in that space) will be removed from the 'top' of the Bottle.
    ///
    /// This will fail if `new_capacity` is greater than `MAX_CAP`
    pub fn resize_in_place(&mut self, new_capacity: usize) -> Result<(), BottleMaxCapError> {
        if new_capacity > MAX_CAP {
            Err(BottleMaxCapError::MaxCapExceeded)
        } else {
            self.capacity = new_capacity;
            if self.content.len() > self.capacity {
                self.content.truncate(self.capacity);
            }
            Ok(())
        }
    }

    /// Creates a new Bottle with the same content as this Bottle, but a different capacity.
    ///
    /// If `new_capacity` is larger than current capacity, empty space will be added to the 'top' of the new Bottle.
    /// If `new_capacity` is smaller than current capacity, space (and any water in that space) will be removed from the 'top' of the new Bottle.
    ///
    /// This will fail if `new_capacity` is greater than `NEW_MAX_CAP`
    pub fn try_get_resized<const NEW_MAX_CAP: usize>(
        &self,
        new_capacity: usize
    ) -> Result<Bottle<NEW_MAX_CAP>, BottleMaxCapError> {
        // number of elements to copy is either the new capacity or the current length of our content (whichever is smaller)
        let len_to_copy = if new_capacity <= self.content.len() {
            new_capacity
        } else {
            self.content.len()
        };
        // the portion of our content to copy
        let content_to_copy = &self.content[..len_to_copy];

        let mut new_content = Vec::new();
        new_content
            .extend_from_slice(content_to_copy)
            .map_err(|_| BottleMaxCapError::MaxCapExceeded)?;

        Ok(Bottle {
            capacity: new_capacity,
            content: new_content
        })
    }

    /// Creates a new Bottle with the same content as this Bottle, but a different capacity. Avoids
    /// copying Bottle content by consuming (taking) this Bottle. Useful for method chaining.
    ///
    /// If `new_capacity` is larger than current capacity, empty space will be added to the 'top' of the new Bottle.
    /// If `new_capacity` is smaller than current capacity, space (and any water in that space) will be removed from the 'top' of the new Bottle.
    ///
    /// If you want to set a new `MAX_CAP`, see [Bottle::try_get_resized].
    ///
    /// This will fail if `new_capacity` is greater than `MAX_CAP`.
    pub fn try_take_as_resized(self, new_capacity: usize) -> Result<Self, BottleMaxCapError> {
        if new_capacity > MAX_CAP {
            return Err(BottleMaxCapError::MaxCapExceeded);
        }

        // take our content as mutable
        let mut new_content = self.content;
        // truncate content if needed, same as resize_in_place
        if new_content.len() > new_capacity {
            new_content.truncate(new_capacity);
        }

        Ok(Bottle {
            capacity: new_capacity,
            content: new_content
        })
    }

    /// Returns the [ColoredWaterUnit] at the top of this bottle
    ///
    /// This returns [None] if there isn't any water in the bottle.
    pub fn get_top_color(&self) -> Option<ColoredWaterUnit> {
        self.content.last().copied()
    }

    /// Returns the [ColoredWaterRun] at the top of this bottle
    ///
    /// This returns [None] if there isn't any water in the bottle.
    pub fn get_top_color_run(&self) -> Option<ColoredWaterRun> {
        if let Some(top_color) = self.get_top_color() {
            let mut color_count: usize = 0;
            for color in self.content.iter().rev() {
                if *color == top_color {
                    color_count += 1;
                } else {
                    break;
                }
            }
            Some(ColoredWaterRun {
                color: top_color,
                size: color_count
            })
        } else {
            None
        }
    }

    /// Attempt to pour a [ColoredWaterRun] into this Bottle.
    ///
    /// If this is successful, will return a new [ColoredWaterRun] representing the portion of
    /// the given `content_to_pour` that wouldn't fit into this Bottle. The `size` of this returned [ColoredWaterRun]
    /// may be 0; this indicates that the entirity of `content_to_pour` fit into this Bottle.
    ///
    /// If this is unsuccessful (i.e., none of the `content_to_pour` could fit into this Bottle/the colors are mismatched),
    /// an [Err] is returned with an appropriate [PourInError] variant. No change is made to this Bottle in this case.
    pub fn try_pour_in(
        &mut self,
        content_to_pour: ColoredWaterRun
    ) -> Result<ColoredWaterRun, PourInError> {
        if let Some(top_color) = self.get_top_color() {
            if top_color != content_to_pour.color {
                return Err(PourInError::MismatchedColors);
            }
        }

        // Add ColoredWaterUnits to this Bottle for each unit in content_to_pour
        let mut count_poured = 0;
        for _ in 0..content_to_pour.size {
            // If we are at this bottle's capacity, stop pouring and record how many units were poured.
            if self.content.len() >= self.capacity {
                break;
            }
            // If we are not yet at this bottle's capacity, pour one additional unit.
            self.content.push(content_to_pour.color).unwrap();
            count_poured += 1;
        }

        if count_poured == 0 {
            Err(PourInError::AlreadyFull)
        } else {
            Ok(ColoredWaterRun {
                color: content_to_pour.color,
                size: content_to_pour.size.saturating_sub(count_poured)
            })
        }
    }

    /// Determine if running [Bottle::try_pour_in] would succeed given the current content of this bottle
    /// and the provided `content_to_pour`, but don't actually modify the content of this bottle.
    ///
    /// Return value is the same as the return value of [Bottle::try_pour_in] would be if called on this same
    /// bottle with the same `content_to_pour`
    pub fn test_pour_in(
        &self,
        content_to_pour: ColoredWaterRun
    ) -> Result<ColoredWaterRun, PourInError> {
        if let Some(top_color) = self.get_top_color() {
            if top_color != content_to_pour.color {
                return Err(PourInError::MismatchedColors);
            }
        }

        let empty_space = self.capacity - self.content.len();
        if empty_space == 0 {
            return Err(PourInError::AlreadyFull);
        }

        // we have verified this pour would work, now calculate the number
        // of units in content_to_pour that we can't accept and return
        Ok(ColoredWaterRun {
            color: content_to_pour.color,
            size: content_to_pour.size.saturating_sub(empty_space)
        })
    }

    /// Attempt to pour a [ColoredWaterRun] out of this Bottle.
    ///
    /// If this is successful, will return `Ok(())`.
    ///
    /// If this is unsuccessful (i.e., this bottle is empty/the destination bottle couldn't accept the pour),
    /// an [Err] is returned with an appropriate [PourOutError] variant. No change is made to either this Bottle
    /// or the destination Bottle in this case.
    pub fn try_pour_out<const OTHER_MAX_CAP: usize>(
        &mut self,
        destination: &mut Bottle<OTHER_MAX_CAP>
    ) -> Result<(), PourOutError> {
        if let Some(run_to_pour) = self.get_top_color_run() {
            let remaining_part_of_run = destination.try_pour_in(run_to_pour)?;

            //Given how many units we tried to pour and how many units couldn't be poured, find the number of units that were actually poured
            let units_poured = run_to_pour.size - remaining_part_of_run.size;

            //Remove that number of units
            self.content.truncate(self.content.len() - units_poured);

            Ok(())
        } else {
            Err(PourOutError::Empty)
        }
    }

    /// Determine if running [Bottle::try_pour_out] would succeed given the current content of this bottle
    /// and the provided `destination` Bottle, but don't actually modify the content of either bottle.
    ///
    /// Return value is the same as the return value of [Bottle::try_pour_out] would be if called on this same
    /// bottle with the same `destination` Bottle.
    pub fn test_pour_out<const OTHER_MAX_CAP: usize>(
        &self,
        destination: &Bottle<OTHER_MAX_CAP>
    ) -> Result<(), PourOutError> {
        if let Some(run_to_pour) = self.get_top_color_run() {
            match destination.test_pour_in(run_to_pour) {
                Ok(_) => Ok(()),
                Err(e) => Err(e.into())
            }
        } else {
            Err(PourOutError::Empty)
        }
    }

    /// Determine if this Bottle is in its final state
    ///
    /// "Final state" in this context means the bottle is in its final possible state:
    /// entirely full of a single color.
    ///
    /// Note: entirely empty bottles are not considered in their final state by this function; although
    /// they can be a part of a finished [GameState](crate::gamestate::GameState), they
    /// can also still be poured into (and therefore, are not definitively in their final state).
    pub fn is_in_final_state(&self) -> bool {
        ColoredWaterRun::try_from(self.get_content()).is_ok_and(|run| run.size == self.capacity)
    }
}

// We need to implement Ord on bottles so that we can sort them into some standardized order.
impl<const MAX_CAP: usize> Ord for Bottle<MAX_CAP> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // bottle A is less than another bottle B when
        // any of the following is true:
        // - bottle A has less capacity
        // - bottle A has fewer units (more empty spaces)
        // - bottle A's bottom unit is a color 'less than' bottle B's bottom unit
        // - the above, for each unit up
        // note that 'less than' for a color means it appears earlier in the ColoredWaterUnit definition

        let cmp_res = self.capacity.cmp(&other.capacity);
        if cmp_res != Ordering::Equal {
            return cmp_res;
        }

        let cmp_res = self.content.len().cmp(&other.content.len());
        if cmp_res != Ordering::Equal {
            return cmp_res;
        }

        for (index, &color) in self.content.iter().enumerate() {
            let other_color = *other.content.get(index).unwrap();
            let cmp_res = (color as u8).cmp(&(other_color as u8));
            if cmp_res != Ordering::Equal {
                return cmp_res;
            }
        }

        Ordering::Equal
    }
}

impl<const MAX_CAP: usize> PartialOrd for Bottle<MAX_CAP> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

///Reasons that setting a [ColoredWaterUnit] within a [Bottle] may fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSetError {
    /// Attempted to set a color at a location that has empty space below it
    EmptyBelow,

    /// Attempted to set a color to empty at a location that has non-empty space above it
    FullAbove,

    /// Attempted to set a color to a location beyond the capacity of the destination [Bottle]
    ExceedsCapacity
}

///Reasons that pouring a [ColoredWaterRun] into a [Bottle] may fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PourInError {
    /// The destination [Bottle] is full and cannot accept any part of the [ColoredWaterRun]
    AlreadyFull,

    /// The destination [Bottle] has a top color that does not match the color of the [ColoredWaterRun]
    MismatchedColors
}

///Reasons that pouring a [ColoredWaterRun] out of a [Bottle] may fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PourOutError {
    /// The source [Bottle] is entirely empty and has no content to pour
    Empty,

    /// The destination [Bottle] could not accept the content to pour
    DestinationError(PourInError)
}

impl From<PourInError> for PourOutError {
    fn from(value: PourInError) -> Self {
        PourOutError::DestinationError(value)
    }
}

/// Create a bottle with some content and an optional size
///
/// There are four forms:
///
/// - Defining a bottle with its content, a capacity, and an explicit max capacity:
///   `bottle!([<Color>, <Color>, ...], <capacity>, <max capacity>)`
///
/// - Definining a bottle with its content and a capacity:
///   `bottle!([<Color>, <Color>, ...], <capacity>)`
///
/// - Defining a bottle with its content and allowing its capacity to match:
///   `bottle!([<Color>, <Color>, ...])`
///
/// - Definining a bottle with its content and allowing its capacity to match,
///   omitting square brackets `[]` surrounding color definitions:
///   `bottle!(<Color>, <Color>, ...)`
///
/// In each form, `<Color>` may be any variant of [ColoredWaterUnit], though
/// you can (and in fact must) omit the `ColoredWaterUnit::` snippet.
/// As few as 1 ColoredWaterUnits may be used, and there is no upper bound.
///
/// Examples:
/// ```
/// use colorgetter::bottle;
/// use colorgetter::bottle::Bottle;
/// use colorgetter::colored_water::ColoredWaterUnit;
///
/// // Bottle defined with content, explicit capacity, and explicit max capacity (allows us to forgo type hinting our variable)
/// let sized_bottle1 = bottle!([Red, Green, Yellow], 4, 5);
/// assert_eq!(
///     sized_bottle1.get_content(),
///     [ColoredWaterUnit::Red, ColoredWaterUnit::Green, ColoredWaterUnit::Yellow]
/// );
///
/// assert_eq!(sized_bottle1.get_capacity(), 4);
/// assert_eq!(sized_bottle1.get_max_capacity(), 5);
///
/// // Bottle defined with content and explicit capacity
/// let sized_bottle2: Bottle<4> = bottle!([Red, Green, Yellow], 4);
/// assert_eq!(
///     sized_bottle2.get_content(),
///     [ColoredWaterUnit::Red, ColoredWaterUnit::Green, ColoredWaterUnit::Yellow]
/// );
/// assert_eq!(sized_bottle2.get_capacity(), 4);
///
/// // Bottle defined with content only
/// let unsized_bottle1: Bottle<4> = bottle!([Red, Green, Yellow]);
/// assert_eq!(
///     unsized_bottle1.get_content(),
///     [ColoredWaterUnit::Red, ColoredWaterUnit::Green, ColoredWaterUnit::Yellow]
/// );
/// assert_eq!(unsized_bottle1.get_capacity(), 3);
///
/// // Bottle defined with content only, omitting square brackets
/// let unsized_bottle2: Bottle<4> = bottle!(Red, Green, Yellow);
/// assert_eq!(
///     unsized_bottle2.get_content(),
///     [ColoredWaterUnit::Red, ColoredWaterUnit::Green, ColoredWaterUnit::Yellow]
/// );
/// assert_eq!(unsized_bottle2.get_capacity(), 3);
/// ```
#[macro_export]
macro_rules! bottle {
    ([$($color:ident),*], $capacity:expr, $max_capacity:expr) => {
        Bottle::<$max_capacity>::try_with_content(&[$(ColoredWaterUnit::$color),*]).unwrap().try_take_as_resized($capacity).unwrap()
    };
    ([$($color:ident),*], $capacity:expr) => {
        Bottle::try_with_content(&[$(ColoredWaterUnit::$color),*]).unwrap().try_take_as_resized($capacity).unwrap()
    };
    ([$($color:ident),+]) => {
        Bottle::try_with_content(&[$(ColoredWaterUnit::$color),+]).unwrap()
    };
    ($($color:ident),+) => {
        bottle!([$($color),+])
    }
}

/// Create an array of [ColoredWaterUnit]s in a more compact form;
/// useful for quickly defining the content of a Bottle.
///
/// To match [bottle!], comes in 2 forms:
///
/// - Defining water colors with square brackets:
///   `bottle_content!([<Color>, <Color>, ...])`
///
/// - Defining water colors without square brackets:
///   `bottle_content!(<Color>, <Color>, ...)`
///
/// In each form, `<Color>` may be any variant of [ColoredWaterUnit], though
/// you can (and in fact must) omit the `ColoredWaterUnit::` snippet.
/// As few as 1 ColoredWaterUnits may be used, and there is no upper bound.
///
/// Examples:
/// ```
/// use colorgetter::bottle_content;
/// use colorgetter::colored_water::ColoredWaterUnit;
///
/// let first_form = bottle_content!([Red, Green, Yellow]);
/// assert_eq!(
///     first_form,
///     [ColoredWaterUnit::Red, ColoredWaterUnit::Green, ColoredWaterUnit::Yellow]
/// );
///
/// let second_form = bottle_content!(Orange, Lime, Pink);
/// assert_eq!(
///     second_form,
///     [ColoredWaterUnit::Orange, ColoredWaterUnit::Lime, ColoredWaterUnit::Pink]
/// );
/// ```
#[macro_export]
macro_rules! bottle_content {
    ($($color:ident),+) => {
        [$(ColoredWaterUnit::$color),+]
    };
    ([$($color:ident),+]) => {
        [$(ColoredWaterUnit::$color),+]
    };
}
