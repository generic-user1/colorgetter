//! Implementation of a KnownBottle; a [Bottle] with all known colors.

use super::{
    Bottle, BottleCapacityError, BottleMaxCapError, BottleSampleResult, ColorSetError,
    PartialBottle, PourInError, PourOutError
};
use crate::colored_water::{ColoredWaterRun, ColoredWaterUnit, PartialColoredWaterUnit};
use heapless::Vec;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Debug};

/// One Bottle that may contain [ColoredWaterUnit]s, all of which are known.
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
#[serde(try_from = "UncheckedKnownBottle<MAX_CAP>")]
pub struct KnownBottle<const MAX_CAP: usize> {
    capacity: usize,
    content: Vec<ColoredWaterUnit, MAX_CAP>
}

/// A KnownBottle that is directly deserializable but has no guarantee that
/// the capacity matches the content. Can be converted into a normal [KnownBottle]
/// with the [TryFrom]/[TryInto] traits.
#[derive(Deserialize)]
struct UncheckedKnownBottle<const MAX_CAP: usize> {
    capacity: usize,
    content: Vec<ColoredWaterUnit, MAX_CAP>
}
impl<const MAX_CAP: usize> TryFrom<UncheckedKnownBottle<MAX_CAP>> for KnownBottle<MAX_CAP> {
    type Error = BottleCapacityError;
    fn try_from(value: UncheckedKnownBottle<MAX_CAP>) -> Result<Self, Self::Error> {
        if value.content.len() <= value.capacity {
            Ok(KnownBottle {
                capacity: value.capacity,
                content: value.content
            })
        } else {
            Err(BottleCapacityError::CapExceeded)
        }
    }
}

impl<const MAX_CAP: usize> KnownBottle<MAX_CAP> {
    /// Tries to create a new, empty KnownBottle
    ///
    /// This will fail if the given `capacity` is greater than `MAX_CAP`
    pub const fn try_new(capacity: usize) -> Result<Self, BottleMaxCapError> {
        if capacity <= MAX_CAP {
            Ok(KnownBottle {
                capacity,
                content: Vec::new()
            })
        } else {
            Err(BottleMaxCapError::MaxCapExceeded)
        }
    }

    /// Tries to create a new KnownBottle with the given content. Capacity is set to the number of elements in `content`.
    ///
    /// This will fail if the size of `content` is greater than `MAX_CAP`
    pub fn try_with_content(content: &[ColoredWaterUnit]) -> Result<Self, BottleMaxCapError> {
        Ok(KnownBottle {
            capacity: content.len(),
            content: Vec::from_slice(content).map_err(|_| BottleMaxCapError::MaxCapExceeded)?
        })
    }

    /// Immutably borrow the content of this KnownBottle.
    ///
    /// Note that the length of the return value may be less than the capacity of this Bottle,
    /// though it will never be greater.
    pub const fn get_content(&self) -> &Vec<ColoredWaterUnit, MAX_CAP> {
        &self.content
    }

    /// Take the content of this KnownBottle
    /// Note that the length of the return value may be less than the capacity of this Bottle,
    /// though it will never be greater.
    pub fn take_content(self) -> Vec<ColoredWaterUnit, MAX_CAP> {
        self.content
    }

    /// Return the maximum capacity of this KnownBottle
    pub const fn get_max_capacity(&self) -> usize {
        MAX_CAP
    }

    /// Sets a new capacity for this KnownBottle.
    ///
    /// If `new_capacity` is larger than current capacity, empty space will be added to the 'top' of the KnownBottle.
    /// If `new_capacity` is smaller than current capacity, space will be removed from the 'top' of the KnownBottle.
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

    /// Creates a new KnownBottle with the same content as this KnownBottle, but a different capacity.
    ///
    /// If `new_capacity` is larger than current capacity, empty space will be added to the 'top' of the new KnownBottle.
    /// If `new_capacity` is smaller than current capacity, space (and any water in that space) will be removed from the 'top' of the new KnownBottle.
    ///
    /// This will fail if `new_capacity` is greater than `NEW_MAX_CAP`
    pub fn try_get_resized<const NEW_MAX_CAP: usize>(
        &self,
        new_capacity: usize
    ) -> Result<KnownBottle<NEW_MAX_CAP>, BottleMaxCapError> {
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

        Ok(KnownBottle {
            capacity: new_capacity,
            content: new_content
        })
    }

    /// Creates a new KnownBottle with the same content as this KnownBottle, but a different capacity. Avoids
    /// copying KnownBottle content by consuming (taking) this KnownBottle. Useful for method chaining.
    ///
    /// If `new_capacity` is larger than current capacity, empty space will be added to the 'top' of the new KnownBottle.
    /// If `new_capacity` is smaller than current capacity, space (and any water in that space) will be removed from the 'top' of the new KnownBottle.
    ///
    /// If you want to set a new `MAX_CAP`, see [KnownBottle::try_get_resized].
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

        Ok(KnownBottle {
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

    /// Attempt to pour a [ColoredWaterRun] into this bottle.
    ///
    /// If this is successful, will return a new [ColoredWaterRun] representing the portion of
    /// the given `content_to_pour` that wouldn't fit into this bottle. The `size` of this returned [ColoredWaterRun]
    /// may be 0; this indicates that the entirity of `content_to_pour` fit into this bottle.
    ///
    /// If this is unsuccessful (i.e., none of the `content_to_pour` could fit into this bottle/the colors are mismatched),
    /// an [Err] is returned with an appropriate [PourInError] variant. No change is made to this bottle in this case.
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

    /// Determine if running [KnownBottle::try_pour_in] would succeed given the current content of this bottle
    /// and the provided `content_to_pour`, but don't actually modify the content of this bottle.
    ///
    /// Return value is the same as the return value of [KnownBottle::try_pour_in] would be if called on this same
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

    /// Attempt to pour a [ColoredWaterRun] out of this bottle.
    ///
    /// If this is successful, will return `Ok(())`.
    ///
    /// If this is unsuccessful (i.e., this bottle is empty/the destination bottle couldn't accept the pour),
    /// an [Err] is returned with an appropriate [PourOutError] variant. No change is made to either this bottle
    /// or the destination bottle in this case.
    pub fn try_pour_out<const OTHER_MAX_CAP: usize>(
        &mut self,
        destination: &mut KnownBottle<OTHER_MAX_CAP>
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

    /// Determine if running [KnownBottle::try_pour_out] would succeed given the current content of this bottle
    /// and the provided `destination` bottle, but don't actually modify the content of either bottle.
    ///
    /// Return value is the same as the return value of [KnownBottle::try_pour_out] would be if called on this same
    /// bottle with the same `destination` bottle.
    pub fn test_pour_out<const OTHER_MAX_CAP: usize>(
        &self,
        destination: &KnownBottle<OTHER_MAX_CAP>
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

    /// Determine if this KnownBottle is in its final state
    ///
    /// "Final state" in this context means the bottle is in its final possible state:
    /// entirely full of a single color.
    ///
    /// Note: entirely empty bottles are not considered in their final state by this function; although
    /// they can be a part of a finished [KnownGameState](crate::gamestate::KnownGameState), they
    /// can also still be poured into (and therefore, are not definitively in their final state).
    pub fn is_in_final_state(&self) -> bool {
        ColoredWaterRun::try_from(self.get_content()).is_ok_and(|run| run.size == self.capacity)
    }
}

impl<const MAX_CAP: usize> Bottle for KnownBottle<MAX_CAP> {
    /// Try to set the [ColoredWaterUnit] at index `idx` within this bottle to the given `new_color`
    ///
    /// Note that although the [Bottle] trait requires this method accept a [PartialColoredWaterUnit],
    /// unknown colors (i.e. [PartialColoredWaterUnit::UnknownColor]) are not supported and will always result in
    /// [ColorSetError::UnknownNotSupported]
    fn try_set_color(
        &mut self,
        idx: usize,
        new_color: Option<PartialColoredWaterUnit>
    ) -> Result<(), ColorSetError> {
        let new_color: Option<ColoredWaterUnit> = if let Some(new_color) = new_color {
            Some(
                new_color
                    .try_into()
                    .map_err(|_| ColorSetError::UnknownNotSupported)?
            )
        } else {
            None
        };

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

    fn capacity(&self) -> usize {
        self.capacity
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

    fn get_top_content_idx(&self) -> Option<usize> {
        //either we have content and the answer is one minus our length,
        //or we don't have content and we want to return None
        self.get_content().len().checked_sub(1)
    }
}

// We need to implement Ord on bottles so that we can sort them into some standardized order.
impl<const MAX_CAP: usize> Ord for KnownBottle<MAX_CAP> {
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

impl<const MAX_CAP: usize> PartialOrd for KnownBottle<MAX_CAP> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<const MAX_CAP: usize> TryFrom<PartialBottle<MAX_CAP>> for KnownBottle<MAX_CAP> {
    type Error = PartialBottleConversionError;
    fn try_from(value: PartialBottle<MAX_CAP>) -> Result<Self, Self::Error> {
        if value.get_unknown_count() > 0 {
            Err(PartialBottleConversionError::UnknownUnits)
        } else {
            Ok(KnownBottle {
                capacity: value.capacity(),
                content: value.take_known_content()
            })
        }
    }
}
/// Reasons converting from a [PartialBottle] to a [KnownBottle] may fail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartialBottleConversionError {
    /// The [PartialBottle] contains one or more units of unknown color
    UnknownUnits
}
