//! Implementation of a bottle for colored water

use crate::colored_water::{ColoredWaterRun, ColoredWaterUnit};

#[cfg(test)]
mod bottle_tests;

/// One Bottle that may contain [ColoredWaterUnit]s
///
/// Each bottle has a capacity, and some content. The capacity is the number of water units the bottle is allowed to hold,
/// and the content is the different water units that the bottle is currently holding.
///
/// Note that the content may be shorter than the capacity (meaning the bottle has space for more content), but
/// will never be longer.
#[derive(Debug, Clone)]
pub struct Bottle {
    capacity: usize,
    content: Vec<ColoredWaterUnit>
}

impl Bottle {
    /// Creates a new, empty Bottle
    pub fn new(capacity: usize) -> Self {
        Bottle {
            capacity,
            content: Vec::new()
        }
    }

    /// Creates a new Bottle with the given content. Capacity is set to the number of elements in `content`.
    pub fn with_content(content: &[ColoredWaterUnit]) -> Self {
        Bottle {
            capacity: content.len(),
            content: content.to_vec()
        }
    }

    /// Immutably borrow the content of this Bottle.
    ///
    /// Note that the length of the return value may be less than the capacity of this Bottle,
    /// though it will never be greater.
    pub fn get_content(&self) -> &[ColoredWaterUnit] {
        &self.content
    }

    /// Return the capacity of this Bottle.
    pub fn get_capacity(&self) -> usize {
        self.capacity
    }

    /// Sets a new capacity for this Bottle.
    ///
    /// If `new_capacity` is larger than current capacity, empty space will be added to the 'top' of the Bottle.
    /// If `new_capacity` is smaller than current capacity, space (and any water in that space) will be removed from the 'top' of the Bottle.
    pub fn resize_in_place(&mut self, new_capacity: usize) {
        self.capacity = new_capacity;
        if self.content.len() > self.capacity {
            self.content.truncate(self.capacity);
        }
    }

    /// Creates a new Bottle with the same content as this Bottle, but a different capacity.
    ///
    /// If `new_capacity` is larger than current capacity, empty space will be added to the 'top' of the new Bottle.
    /// If `new_capacity` is smaller than current capacity, space (and any water in that space) will be removed from the 'top' of the new Bottle.
    pub fn get_resized(&self, new_capacity: usize) -> Self {
        // number of elements to copy is either the new capacity or the current length of our content (whichever is smaller)
        let len_to_copy = if new_capacity <= self.content.len() {
            new_capacity
        } else {
            self.content.len()
        };
        // the portion of our content to copy
        let content_to_copy = &self.content[..len_to_copy];

        let mut new_content = Vec::with_capacity(content_to_copy.len());
        new_content.extend_from_slice(content_to_copy);

        Bottle {
            capacity: new_capacity,
            content: new_content
        }
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
            self.content.push(content_to_pour.color);
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

    /// Attempt to pour a [ColoredWaterRun] out of this Bottle.
    ///
    /// If this is successful, will return `Ok(())`.
    ///
    /// If this is unsuccessful (i.e., this bottle is empty/the destination bottle couldn't accept the pour),
    /// an [Err] is returned with an appropriate [PourOutError] variant. No change is made to either this Bottle
    /// or the destination Bottle in this case.
    pub fn try_pour_out(&mut self, destination: &mut Bottle) -> Result<(), PourOutError> {
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
