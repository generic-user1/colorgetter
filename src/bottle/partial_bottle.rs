//! Implementation of a PartialBottle; a [Bottle] with partially unknown colors.

use crate::colored_water::{ColoredWaterUnit, PartialColoredWaterUnit};
use heapless::Vec;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use super::{
    Bottle, BottleCapacityError, BottleMaxCapError, BottleSampleResult, ColorSetError, KnownBottle
};

/// One Bottle that may contain [ColoredWaterUnit]s, but where the specific color of some
/// of the units is unknown.
///
/// Each bottle has a capacity, and some content. The capacity is the number of water units the bottle is allowed to hold,
/// and the content is the different water units that the bottle is currently holding. PartialBottle also tracks an "unknown count";
/// the number of capacity units that are filled with some color, but where that color is not known. Unknown units are always at the bottom of the bottle.
///
/// Note that the content may be shorter than the capacity (meaning the bottle has space for more content), but
/// will never be longer. Additionally, since the unknown count takes up capacity, there can only be `capacity - unknown_count` known units
/// of content.
///
/// For performance reasons, PartialBottles must not require heap allocations. To this end, each bottle has a `MAX_CAP`; this is
/// the maximum capacity that particular bottle can have. Their actual capacity can be any value at or below `MAX_CAP`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(try_from = "UncheckedPartialBottle<MAX_CAP>")]
pub struct PartialBottle<const MAX_CAP: usize> {
    capacity: usize,
    content: Vec<ColoredWaterUnit, MAX_CAP>,
    unknown_count: usize
}

/// A PartialBottle that is directly deserializable but has no guarantee that
/// the capacity matches the content. Can be converted into a normal [PartialBottle]
/// with the [TryFrom]/[TryInto] traits.
#[derive(Deserialize)]
struct UncheckedPartialBottle<const MAX_CAP: usize> {
    capacity: usize,
    content: Vec<ColoredWaterUnit, MAX_CAP>,

    #[serde(default)]
    unknown_count: usize
}
impl<const MAX_CAP: usize> TryFrom<UncheckedPartialBottle<MAX_CAP>> for PartialBottle<MAX_CAP> {
    type Error = BottleCapacityError;
    fn try_from(value: UncheckedPartialBottle<MAX_CAP>) -> Result<Self, Self::Error> {
        if (value.content.len() + value.unknown_count) <= value.capacity {
            Ok(PartialBottle {
                capacity: value.capacity,
                content: value.content,
                unknown_count: value.unknown_count
            })
        } else {
            Err(BottleCapacityError::CapExceeded)
        }
    }
}

impl<const MAX_CAP: usize> PartialBottle<MAX_CAP> {
    /// Tries to create a new, empty PartialBottle.
    ///
    /// `capacity` is the capacity of the bottle to create. `unknown_count`
    /// is the number of units of that capacity to treat as unknown colors.
    ///
    /// This will fail if:
    /// - The given `capacity` is greater than `MAX_CAP` (results in [BottleCapacityError::MaxCapExceeded])
    /// - The given `unknown_count` is greater than `MAX_CAP` (results in [BottleCapacityError::MaxCapExceeded])
    /// - The given `unknown_count` is greater than the given `capacity` (results in [BottleCapacityError::CapExceeded])
    pub const fn try_new(
        capacity: usize,
        unknown_count: usize
    ) -> Result<Self, BottleCapacityError> {
        if capacity > MAX_CAP || unknown_count > MAX_CAP {
            Err(BottleCapacityError::MaxCapExceeded)
        } else if unknown_count > capacity {
            Err(BottleCapacityError::CapExceeded)
        } else {
            Ok(PartialBottle {
                capacity,
                unknown_count,
                content: Vec::new()
            })
        }
    }

    /// Tries to create a new PartialBottle with the given content and unknown count.
    /// Capacity is set to the number of elements in `content` plus the unknown count.
    ///
    /// This will fail if the size of `content` plus the `unknown_count` is greater than `MAX_CAP`
    pub fn try_with_content(
        content: &[ColoredWaterUnit],
        unknown_count: usize
    ) -> Result<Self, BottleMaxCapError> {
        let content = Vec::from_slice(content).map_err(|_| BottleMaxCapError::MaxCapExceeded)?;
        if content.len() + unknown_count > MAX_CAP {
            Err(BottleMaxCapError::MaxCapExceeded)
        } else {
            Ok(PartialBottle {
                capacity: content.len() + unknown_count,
                content,
                unknown_count
            })
        }
    }

    /// Immutably borrow the known content of this PartialBottle. Does not include unknown content.
    ///
    /// Note that the length of the return value may be less than the capacity of this PartialBottle,
    /// though it will never be greater - specifically, it must be somewhere between 0 and `capacity - unknown_count`
    /// units long.
    pub const fn get_known_content(&self) -> &Vec<ColoredWaterUnit, MAX_CAP> {
        &self.content
    }

    /// Take the known content of this PartialBottle. Does not include unknown content.
    ///
    /// Note that the length of the return value may be less than the capacity of this PartialBottle,
    /// though it will never be greater - specifically, it must be somewhere between 0 and `capacity - unknown_count`
    /// units long.
    pub fn take_known_content(self) -> Vec<ColoredWaterUnit, MAX_CAP> {
        self.content
    }

    /// Return the maximum capacity of this PartialBottle
    pub const fn get_max_capacity(&self) -> usize {
        MAX_CAP
    }

    /// Return the unknown count of this PartialBottle
    pub const fn get_unknown_count(&self) -> usize {
        self.unknown_count
    }

    /// Sets a new capacity for this PartialBottle.
    ///
    /// If `new_capacity` is larger than current capacity, empty space will be added to the 'top' of the PartialBottle.
    /// If `new_capacity` is smaller than current capacity, space (and any water in that space, known or unknown)
    /// will be removed from the 'top' of the PartialBottle.
    ///
    /// This will fail if `new_capacity` is greater than `MAX_CAP`
    pub fn resize_in_place(&mut self, new_capacity: usize) -> Result<(), BottleMaxCapError> {
        if new_capacity > MAX_CAP {
            Err(BottleMaxCapError::MaxCapExceeded)
        } else {
            let capacity_for_known = new_capacity.saturating_sub(self.unknown_count);

            if self.content.len() > capacity_for_known {
                self.content.truncate(capacity_for_known);
            }

            // if we have more unknown units than capacity total,
            // set unknown count to capacity (as the known units would already have
            // been removed above)
            self.unknown_count = self.unknown_count.min(new_capacity);

            self.capacity = new_capacity;
            Ok(())
        }
    }

    /// Creates a new PartialBottle with the same content as this PartialBottle, but a different capacity.
    ///
    /// If `new_capacity` is larger than current capacity, empty space will be added to the 'top' of the new PartialBottle.
    /// If `new_capacity` is smaller than current capacity, space (and any water in that space, known or unknown)
    /// will be removed from the 'top' of the new PartialBottle.
    ///
    /// This will fail if `new_capacity` is greater than `MAX_CAP`
    pub fn try_get_resized<const NEW_MAX_CAP: usize>(
        &self,
        new_capacity: usize
    ) -> Result<PartialBottle<NEW_MAX_CAP>, BottleMaxCapError> {
        if new_capacity > NEW_MAX_CAP {
            return Err(BottleMaxCapError::MaxCapExceeded);
        }

        let capacity_for_known = new_capacity.saturating_sub(self.unknown_count);
        let new_unknown_count = new_capacity.min(self.unknown_count);

        // number of elements to copy is either the new capacity for known or the current length of our content (whichever is smaller)
        let len_to_copy = capacity_for_known.min(self.content.len());
        // the portion of our content to copy
        let content_to_copy = &self.content[..len_to_copy];

        let mut new_content = Vec::new();
        new_content
            .extend_from_slice(content_to_copy)
            .map_err(|_| BottleMaxCapError::MaxCapExceeded)?;
        // in theory, this error should never occur since the first check we do should catch
        // the relevant condition. in practice, I see no practical harm in keeping this mapping.

        Ok(PartialBottle {
            capacity: new_capacity,
            content: new_content,
            unknown_count: new_unknown_count
        })
    }

    /// Creates a new PartialBottle with the same content as this PartialBottle, but a different capacity. Avoids
    /// copying PartialBottle content by consuming (taking) this PartialBottle. Useful for method chaining.
    ///
    /// If `new_capacity` is larger than current capacity, empty space will be added to the 'top' of the new PartialBottle.
    /// If `new_capacity` is smaller than current capacity, space (and any water in that space, known or unknown)
    /// will be removed from the 'top' of the new PartialBottle.
    ///
    /// If you want to set a new `MAX_CAP`, see [PartialBottle::try_get_resized].
    ///
    /// This will fail if `new_capacity` is greater than `MAX_CAP`
    pub fn try_take_as_resized(self, new_capacity: usize) -> Result<Self, BottleMaxCapError> {
        if new_capacity > MAX_CAP {
            return Err(BottleMaxCapError::MaxCapExceeded);
        }

        let capacity_for_known = new_capacity.saturating_sub(self.unknown_count);

        // take our content as mutable
        let mut new_content = self.content;
        // truncate content if needed, same as resize_in_place
        if new_content.len() > capacity_for_known {
            new_content.truncate(capacity_for_known);
        }

        Ok(PartialBottle {
            capacity: new_capacity,
            content: new_content,
            unknown_count: self.unknown_count.min(new_capacity)
        })
    }
}

impl<const MAX_CAP: usize> Bottle for PartialBottle<MAX_CAP> {
    fn try_set_color(
        &mut self,
        idx: usize,
        new_color: Option<PartialColoredWaterUnit>
    ) -> Result<(), ColorSetError> {
        match new_color {
            None => {
                // we are trying to set some color to empty. this is allowed only when the color we're trying to set
                // is the topmost color (whether it's known or unknown), or when it's one unit above the topmost color
                // (which is a no-op, but not an error)
                if idx < self.unknown_count {
                    //we're trying to set an unknown color - need to determine if it's the topmost or not
                    if (idx + 1) == self.unknown_count {
                        // trying to set the topmost unknown color to empty
                        if self.content.is_empty() {
                            // we have no known colors, so topmost unknown color is top color overall, and this is allowed.
                            // just decrement unknown count by one
                            self.unknown_count = self.unknown_count.saturating_sub(1);
                            Ok(())
                        } else {
                            // we have at least one known color, so topmost unknown color is not top overall, and this
                            // is not allowed
                            Err(ColorSetError::FullAbove)
                        }
                    } else {
                        // trying to set an unknown color to empty when there are unknowns above; not allowed
                        Err(ColorSetError::FullAbove)
                    }
                } else {
                    // trying to set a known color to empty. since we're dealing with known colors,
                    // adjust the index so that idx 0 points to the bottom known color
                    let idx = idx - self.unknown_count;

                    if (idx + 1) == self.content.len() {
                        // trying to set topmost known color to empty; this is allowed
                        self.content.pop();
                        Ok(())
                    } else if idx == self.content.len() {
                        // trying to set color just above topmost known color to empty; this is
                        // a no-op but is allowed
                        Ok(())
                    } else if idx > self.content.len() {
                        // trying to set color more than one above topmost known color to empty;
                        // this is not allowed
                        Err(ColorSetError::EmptyBelow)
                    } else {
                        // trying to set non-topmost known color to empty; this is not allowed
                        Err(ColorSetError::FullAbove)
                    }
                }
            }
            Some(PartialColoredWaterUnit::UnknownColor) => {
                // we are trying to set some color to unknown. this is allowed only when the color we're trying
                // to set is either already unknown, or is one unit above the topmost unknown color.
                if idx < self.unknown_count {
                    // trying to set an unknown color to unknown; no-op, not an error
                    Ok(())
                } else if idx == self.unknown_count {
                    // trying to set the color one above the topmost unknown to unknown.
                    // this is allowed, but only if this wouldn't put us over capacity.
                    if !self.content.is_empty() {
                        //we're trying to set the bottom-most known color to unknown; this is always allowed
                        //since we can't increase capacity usage this way
                        self.content.remove(0);
                        self.unknown_count += 1;
                        Ok(())
                    } else {
                        //we're trying to set a new topmost color to unknown, which increases capacity usage,
                        //so we must ensure this wouldn't put us over capacity.
                        if (self.unknown_count + 1) <= self.capacity {
                            self.unknown_count += 1;
                            Ok(())
                        } else {
                            Err(ColorSetError::ExceedsCapacity)
                        }
                    }
                } else {
                    // trying to set a color more than one above our topmost unknown color to unknown
                    // this is never allowed, but we need to determine whether it's because there's a known color
                    // under us, or empty space. to do that, we'll adjust the index so that 0 points to bottom known color,
                    // then check if there's a known color there or not.
                    let idx = idx - self.unknown_count;
                    if idx < self.content.len() {
                        Err(ColorSetError::KnownBelow)
                    } else {
                        Err(ColorSetError::EmptyBelow)
                    }
                }
            }
            Some(PartialColoredWaterUnit::Color(c)) => {
                // we are trying to set some color to a known color. this is allowed
                // as long as the color we are trying to set is either already known, the topmost unknown color,
                // or one above the topmost color (be it known or unknown)
                if idx < self.unknown_count {
                    // we are trying to set some unknown color to a known color;
                    // check if this is the topmost unknown color
                    if (idx + 1) == self.unknown_count {
                        // we are indeed modifying the topmost unknown color; this is allowed
                        self.unknown_count -= 1;
                        self.content.insert(0, c).unwrap();
                        Ok(())
                    } else {
                        // we are modifying an unknown color that isn't the topmost unknown; this isn't allowed.
                        Err(ColorSetError::UnknownAbove)
                    }
                } else {
                    // we are working with a color that isn't unknown; adjust our index so that 0 points to the first
                    // known color
                    let idx = idx - self.unknown_count;

                    if idx < self.content.len() {
                        // we are modifying some known color into a (possibly different) known color; this is allowed
                        *self.content.get_mut(idx).unwrap() = c;
                        Ok(())
                    } else if idx == self.content.len() {
                        // we are modifying the unit one above the topmost known color; this would increase
                        // capacity usage so we need to make sure we don't go over capacity
                        if (self.unknown_count + self.content.len() + 1) <= self.capacity {
                            self.content.push(c).unwrap();
                            Ok(())
                        } else {
                            Err(ColorSetError::ExceedsCapacity)
                        }
                    } else {
                        // we are modifying a location more than one unit above our current topmost color; this isn't allowed
                        Err(ColorSetError::EmptyBelow)
                    }
                }
            }
        }
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn sample_at(&self, idx: usize) -> BottleSampleResult {
        if idx > self.capacity() {
            BottleSampleResult::OutOfBounds
        } else if idx >= self.get_unknown_count() {
            // adjust idx so that idx 0 points to first known color
            let idx = idx - self.get_unknown_count();
            let sampled = self.get_known_content().get(idx);
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

    fn get_top_content_idx(&self) -> Option<usize> {
        //first, find our overall content length including both known and unknown
        let overall_content_len = self.get_unknown_count() + self.get_known_content().len();
        //then, our answer is our overall content length minus 1, or None if our overall length was zero.
        overall_content_len.checked_sub(1)
    }
}

impl<const MAX_CAP: usize> From<KnownBottle<MAX_CAP>> for PartialBottle<MAX_CAP> {
    fn from(value: KnownBottle<MAX_CAP>) -> Self {
        PartialBottle {
            capacity: value.capacity(),
            content: value.take_content(),
            unknown_count: 0
        }
    }
}
