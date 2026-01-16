//! Definition of the GameStateDisplay trait

use crate::{
    bottle::{Bottle, BottleSampleResult},
    gamestate::{Pour, ValidPourIter}
};
use crossterm::{
    cursor::{MoveDown, MoveLeft, MoveRight},
    style::{ContentStyle, Print, PrintStyledContent, StyledContent, Stylize},
    QueueableCommand
};
use std::{
    hash::Hash,
    io,
    num::NonZeroUsize,
    ops::{Bound, RangeBounds},
    slice::SliceIndex
};

/// Types representing game states; collections of [Bottle]s that can be poured between
pub trait GameState: Clone {
    type BottleT: Bottle;

    /// Gets the bottles of this game state as a slice
    fn get_bottles(&self) -> &[Self::BottleT];

    /// Get the bottles of this game state as a mutable slice
    fn get_mut_bottles(&mut self) -> &mut [Self::BottleT];

    /// Queues the display of this entire game state for the given `ostream` (typically [std::io::stdout])
    ///
    /// `selected`, if provided, specifies a [ColoredWaterUnit](crate::colored_water::ColoredWaterUnit) to display as being selected.
    /// It is interpreted as `(bottleIdx, unitIdx)` where `bottleIdx` is the index of the bottle and `unitIdx` is the index of the
    /// [ColoredWaterUnit](crate::colored_water::ColoredWaterUnit) within that bottle (`0` being the bottom unit).
    /// Providing [None] or out-of-bounds coordinates results in no unit being displayed as selected.
    /// If `bottleIdx` is specified but `unitIdx` isn't, then an entire bottle will be displayed as selected (assuming the `bottleIdx`
    /// is in-bounds).
    ///
    /// `pour`, if provided, specifies a [Pour] to display as having been applied to the game state. If the [Pour::source_bottle_index]
    /// or [Pour::dest_bottle_index] are out of bounds, the source or destination bottles will not be marked. Notably, this does not
    /// check if the [Pour] could be converted into a [ValidPour](crate::gamestate::ValidPour) - it simply marks the indicated bottles as 'F' (from) and 'T' (to).
    ///
    /// Does not flush; the caller must call flush on `ostream` in order to actually display the game state.
    ///
    /// If you want to queue some arbitrary portion of a game state, see [GameState::queue_display_partial]
    fn queue_display_full<T: QueueableCommand>(
        &self,
        ostream: &mut T,
        selected: Option<(usize, Option<usize>)>,
        pour: Option<&Pour>
    ) -> io::Result<()> {
        self.queue_display_partial(ostream, .., selected, pour)
    }

    /// Queues the display of some portion of this game state for the given `ostream` (typically [std::io::stdout])
    /// Only queues for display bottles that are within the specified `range`.
    ///
    /// `selected`, if provided, specifies a [ColoredWaterUnit](crate::colored_water::ColoredWaterUnit) to display as being selected.
    /// It is interpreted as `(bottleIdx, unitIdx)` where `bottleIdx` is the index of the bottle and `unitIdx` is the index of the
    /// [ColoredWaterUnit](crate::colored_water::ColoredWaterUnit) within that bottle (`0` being the bottom unit).
    /// Providing [None] or out-of-bounds coordinates results in no unit being displayed as selected.
    /// If `bottleIdx` is specified but `unitIdx` isn't, then an entire bottle will be displayed as selected (assuming the `bottleIdx`
    /// is in-bounds).
    ///
    /// `pour`, if provided, specifies a [Pour] to display as having been applied to the game state. If the [Pour::source_bottle_index]
    /// or [Pour::dest_bottle_index] are out of bounds, the source or destination bottles will not be marked. Notably, this does not
    /// check if the [Pour] could be converted into a [ValidPour](crate::gamestate::ValidPour) - it simply marks the indicated bottles as 'F' (from) and 'T' (to).
    ///
    /// Does not flush; the caller must call flush on `ostream` in order to actually display the game state.
    ///
    /// If you want to queue this entire game state for display, see [GameState::queue_display_full]
    fn queue_display_partial<
        T: QueueableCommand,
        V: RangeBounds<usize> + SliceIndex<[Self::BottleT], Output = [Self::BottleT]> + Clone
    >(
        &self,
        ostream: &mut T,
        range: V,
        selected: Option<(usize, Option<usize>)>,
        pour: Option<&Pour>
    ) -> io::Result<()> {
        const FILLED: char = '█';
        const EMPTY: char = '░';
        const SELECTED: char = '▓';
        const UNKNOWN: char = '?';
        const FROM: char = 'F';
        const TO: char = 'T';

        let bottles = self.get_bottles();

        //row count is height of highest considered bottle, plus 1 for indicators
        let row_count = bottles[range.clone()]
            .iter()
            .map(|b| b.capacity())
            .max()
            .and_then(|x| x.checked_add(1));

        if let Some(row_count) = row_count {
            for row_index in (0..row_count).rev() {
                for (bottle_offset, bottle) in bottles[range.clone()].iter().enumerate() {
                    let bottle_idx = match range.start_bound() {
                        Bound::Excluded(&base) => base + bottle_offset + 1,
                        Bound::Included(&base) => base + bottle_offset,
                        Bound::Unbounded => bottle_offset
                    };

                    let is_selected = if let Some((sel_b_idx, sel_r_idx)) = selected {
                        bottle_idx == sel_b_idx
                            && if let Some(sel_r_idx) = sel_r_idx {
                                row_index == sel_r_idx
                            } else {
                                true
                            }
                    } else {
                        false
                    };

                    if bottle.capacity() < (row_index + 1) {
                        // if capacity is less than row index + 1 (row_index is 0-based so we add 1 for 1-based),
                        // this bottle doesn't have the capacity to reach here.

                        if let Some(pour) = pour {
                            // if we have a pour to display, and we're 1 row above the source or dest bottle, print
                            // the appropriate indicator
                            if bottle.capacity() == row_index {
                                if bottle_idx == pour.source_bottle_index {
                                    ostream.queue(Print(FROM))?;
                                } else if bottle_idx == pour.dest_bottle_index {
                                    ostream.queue(Print(TO))?;
                                } else {
                                    //this bottle isn't our source or dest!
                                    ostream.queue(MoveRight(1))?;
                                }
                            }
                        } else {
                            ostream.queue(MoveRight(1))?;
                        }
                    } else {
                        match bottle.sample_at(row_index) {
                            BottleSampleResult::KnownColor(c) => {
                                let styled_content = StyledContent::new(
                                    ContentStyle {
                                        foreground_color: Some(c.into()),
                                        ..Default::default()
                                    },
                                    if is_selected { SELECTED } else { FILLED }
                                );
                                ostream.queue(PrintStyledContent(styled_content))?;
                            }
                            BottleSampleResult::UnknownColor => {
                                if is_selected {
                                    ostream
                                        .queue(PrintStyledContent(UNKNOWN.black().on_white()))?;
                                } else {
                                    ostream.queue(Print(UNKNOWN))?;
                                }
                            }
                            BottleSampleResult::Empty => {
                                // bottle has capacity to reach here, but there's no content there
                                ostream.queue(Print(if is_selected { SELECTED } else { EMPTY }))?;
                            }
                            BottleSampleResult::OutOfBounds => {
                                // we shouldn't ever reach here since we already checked that our index was in bounds,
                                // but if we somehow do, just move on without printing anything
                                ostream.queue(MoveRight(1))?;
                            }
                        }
                    }

                    // print an empty space between bottles
                    ostream.queue(MoveRight(1))?;
                }
                //move cursor to beginning of next row

                ostream.queue(MoveLeft((bottles.len() * 2).try_into().unwrap_or(65535)))?;
                ostream.queue(MoveDown(1))?;
            }
        }

        Ok(())
    }

    /// Queues the display of this entire game state for the given `ostream` (typically [std::io::stdout]),
    /// queuing on `row_count` distinct rows of bottles.
    ///
    /// Note that if the number of bottles is not evenly divisible by `row_count`, the size of the final row
    /// may be inconsistent with the size of earlier rows, and there may be fewer rows than `row_count`.
    ///
    /// `selected`, if provided, specifies a [ColoredWaterUnit](crate::colored_water::ColoredWaterUnit) to display as being selected.
    /// It is interpreted as `(bottleIdx, unitIdx)` where `bottleIdx` is the index of the bottle and `unitIdx` is the index of the
    /// [ColoredWaterUnit](crate::colored_water::ColoredWaterUnit) within that bottle (`0` being the bottom unit).
    /// Providing [None] or out-of-bounds coordinates results in no unit being displayed as selected.
    /// If `bottleIdx` is specified but `unitIdx` isn't, then an entire bottle will be displayed as selected (assuming the `bottleIdx`
    /// is in-bounds).
    ///
    /// `pour`, if provided, specifies a [Pour] to display as having been applied to the GameState. If the [Pour::source_bottle_index]
    /// or [Pour::dest_bottle_index] are out of bounds, the source or destination bottles will not be marked. Notably, this does not
    /// check if the [Pour] could be converted into a [ValidPour](crate::gamestate::ValidPour) - it simply marks the indicated bottles as 'F' (from) and 'T' (to).
    ///
    /// Does not flush; the caller must call flush on `ostream` in order to actually display the GameState.
    fn queue_display_rows<T: QueueableCommand>(
        &self,
        ostream: &mut T,
        row_count: NonZeroUsize,
        selected: Option<(usize, Option<usize>)>,
        pour: Option<&Pour>
    ) -> io::Result<()> {
        let bottles = self.get_bottles();

        let row_count = row_count.get();
        // if our row count is 1, just use the full display function as that'll do the job and we don't
        // need to worry about wonky math. from this point on, we can assume that row_count >= 2.
        if row_count == 1 {
            return self.queue_display_full(ostream, selected, pour);
        }

        let row_length = bottles.len().div_ceil(row_count);

        let mut range_start = 0;
        let mut range_end = row_length;
        loop {
            let range = range_start..(range_end.min(bottles.len()));
            self.queue_display_partial(ostream, range, selected, pour)?;

            range_start = range_end;
            range_end += row_length;
            if range_start < bottles.len() {
                ostream.queue(Print('\n'))?;
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Returns an iterator over all [ValidPour](crate::gamestate::ValidPour)s you could apply to this GameState
    fn iter_pours(&self) -> ValidPourIter<'_, Self> {
        ValidPourIter::new(self)
    }
}

/// Types representing [GameState]s that a [Solution](crate::solution::Solution) can be found for
pub trait SolvableGameState: GameState + Eq + Hash + Ord + Send + 'static {
    /// Returns whether this SolvableGameState is solved
    ///
    /// What precisely "solved" means depends on the particular type of SolvableGameState.
    /// When this is true for some state, the solving algorithm will stop and declare it has found a solution.
    fn is_solved(&self) -> bool;
}
