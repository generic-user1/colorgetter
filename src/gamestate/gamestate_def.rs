use super::*;
use crate::bottle::Bottle;
use crossterm::{
    cursor::{MoveDown, MoveLeft, MoveRight},
    style::{ContentStyle, Print, PrintStyledContent, StyledContent},
    QueueableCommand
};
use heapless::Vec;
use std::{
    hash::Hash,
    io,
    num::NonZeroUsize,
    ops::{Bound, RangeBounds},
    slice::SliceIndex,
    usize
};

/// The state a particular game is in
///
/// That is, represents what bottles exist and what order they're in.
#[derive(Debug, Clone, Eq)]
pub struct GameState<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    pub bottles: Vec<Bottle<B_MAX_CAP>, MAX_BCOUNT>
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> GameState<MAX_BCOUNT, B_MAX_CAP> {
    /// Queues the display of this entire GameState for the given `ostream` (typically [std::io::stdout])
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
    /// check if the [Pour] could be converted into a [ValidPour] - it simply marks the indicated [Bottle]s as 'F' (from) and 'T' (to).
    ///
    /// Does not flush; the caller must call flush on `ostream` in order to actually display the GameState.
    ///
    /// If you want to queue some arbitrary portion of this GameState, see [GameState::queue_display_partial]
    pub fn queue_display_full<T: QueueableCommand>(
        &self,
        ostream: &mut T,
        selected: Option<(usize, Option<usize>)>,
        pour: Option<&Pour>
    ) -> io::Result<()> {
        self.queue_display_partial(ostream, .., selected, pour)
    }

    /// Queues the display of some portion of this GameState for the given `ostream` (typically [std::io::stdout])
    /// Only queues for display bottles that are within the specified `range`.
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
    /// check if the [Pour] could be converted into a [ValidPour] - it simply marks the indicated [Bottle]s as 'F' (from) and 'T' (to).
    ///
    /// Does not flush; the caller must call flush on `ostream` in order to actually display the GameState.
    ///
    /// If you want to queue this entire GameState for display, see [GameState::queue_display_full]
    pub fn queue_display_partial<
        T: QueueableCommand,
        V: RangeBounds<usize> + SliceIndex<[Bottle<B_MAX_CAP>], Output = [Bottle<B_MAX_CAP>]> + Clone
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
        const NOTHING: char = ' ';
        const FROM: char = 'F';
        const TO: char = 'T';

        // Cursor movement doesn't appear to work right on Windows when terminal isn't in alternate screen
        const USE_CURSOR: bool = true;

        // Determine row count from max capacity of any bottle in this gamestate (true),
        // or only consider bottles in this range (false)
        const ROW_COUNT_FROM_GLOBAL_MAX: bool = false;

        //row count is height of highest considered bottle, plus 1 for indicators
        let row_count = if ROW_COUNT_FROM_GLOBAL_MAX {
            self.bottles.iter().map(|b| b.get_capacity()).max()
        } else {
            self.bottles[range.clone()]
                .iter()
                .map(|b| b.get_capacity())
                .max()
        }
        .and_then(|x| x.checked_add(1));

        if let Some(row_count) = row_count {
            for row_index in (0..row_count).rev() {
                for (bottle_offset, bottle) in self.bottles[range.clone()].iter().enumerate() {
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

                    if bottle.get_capacity() < (row_index + 1) {
                        // if capacity is less than row index + 1 (row_index is 0-based so we add 1 for 1-based),
                        // this bottle doesn't have the capacity to reach here.

                        if let Some(pour) = pour {
                            // if we have a pour to display, and we're 1 row above the source or dest bottle, print
                            // the appropriate indicator
                            if bottle.get_capacity() == row_index {
                                if bottle_idx == pour.source_bottle_index {
                                    ostream.queue(Print(FROM))?;
                                } else if bottle_idx == pour.dest_bottle_index {
                                    ostream.queue(Print(TO))?;
                                } else {
                                    //this bottle isn't our source or dest!
                                    if USE_CURSOR {
                                        ostream.queue(MoveRight(1))?;
                                    } else {
                                        ostream.queue(Print(NOTHING))?;
                                    }
                                }
                            }
                        } else if USE_CURSOR {
                            ostream.queue(MoveRight(1))?;
                        } else {
                            ostream.queue(Print(NOTHING))?;
                        }
                    } else if let Some(color) = bottle.get_content().get(row_index) {
                        let styled_content = StyledContent::new(
                            ContentStyle {
                                foreground_color: Some(color.into()),
                                ..Default::default()
                            },
                            if is_selected { SELECTED } else { FILLED }
                        );
                        ostream.queue(PrintStyledContent(styled_content))?;
                    } else {
                        // bottle has capacity to reach here, but there's no content there
                        ostream.queue(Print(if is_selected { SELECTED } else { EMPTY }))?;
                    }
                    // print an empty space between bottles
                    if USE_CURSOR {
                        ostream.queue(MoveRight(1))?;
                    } else {
                        ostream.queue(Print(NOTHING))?;
                    }
                }
                //move cursor to beginning of next row
                if USE_CURSOR {
                    ostream.queue(MoveLeft(
                        (self.bottles.len() * 2).try_into().unwrap_or(65535)
                    ))?;
                    ostream.queue(MoveDown(1))?;
                } else {
                    ostream.queue(Print('\n'))?;
                }
            }
        }

        Ok(())
    }

    /// Queues the display of this entire GameState for the given `ostream` (typically [std::io::stdout]),
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
    /// check if the [Pour] could be converted into a [ValidPour] - it simply marks the indicated [Bottle]s as 'F' (from) and 'T' (to).
    ///
    /// Does not flush; the caller must call flush on `ostream` in order to actually display the GameState.
    pub fn queue_display_rows<T: QueueableCommand>(
        &self,
        ostream: &mut T,
        row_count: NonZeroUsize,
        selected: Option<(usize, Option<usize>)>,
        pour: Option<&Pour>
    ) -> io::Result<()> {
        let row_count = row_count.get();
        // if our row count is 1, just use the full display function as that'll do the job and we don't
        // need to worry about wonky math. from this point on, we can assume that row_count >= 2.
        if row_count == 1 {
            return self.queue_display_full(ostream, selected, pour);
        }

        let row_length = (self.bottles.len() + (row_count - 1)) / row_count;

        let mut range_start = 0;
        let mut range_end = row_length;
        loop {
            let range = range_start..(range_end.min(self.bottles.len()));
            self.queue_display_partial(ostream, range, selected, pour)?;

            range_start = range_end;
            range_end += row_length;
            if range_start < self.bottles.len() {
                ostream.queue(Print('\n'))?;
            } else {
                break;
            }
        }

        Ok(())
    }
    /// Returns whether this GameState represents a finished game
    ///
    /// The game is finished when all bottles are either completely empty or
    /// completely full of a single color
    pub fn is_finished(&self) -> bool {
        for bottle in &self.bottles {
            if !(bottle.is_in_final_state() || bottle.get_content().is_empty()) {
                return false;
            }
        }
        true
    }

    /// Returns an iterator over all [ValidPour](crate::gamestate::ValidPour)s you could apply to this GameState
    pub fn iter_pours(&self) -> ValidPourIter<MAX_BCOUNT, B_MAX_CAP> {
        ValidPourIter::new(self)
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> Ord for GameState<MAX_BCOUNT, B_MAX_CAP> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let mut our_std_order_bottles = self.bottles.clone();
        our_std_order_bottles.sort();

        let mut other_std_order_bottles = other.bottles.clone();
        other_std_order_bottles.sort();

        our_std_order_bottles.cmp(&other_std_order_bottles)
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> PartialOrd
    for GameState<MAX_BCOUNT, B_MAX_CAP>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> PartialEq
    for GameState<MAX_BCOUNT, B_MAX_CAP>
{
    fn eq(&self, other: &Self) -> bool {
        if self.is_finished() != other.is_finished() {
            return false;
        }

        let mut our_std_order_bottles = self.bottles.clone();
        our_std_order_bottles.sort();

        let mut other_std_order_bottles = other.bottles.clone();
        other_std_order_bottles.sort();

        our_std_order_bottles == other_std_order_bottles
    }
}
impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> Hash for GameState<MAX_BCOUNT, B_MAX_CAP> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Include whether we're finished
        self.is_finished().hash(state);

        // Hash for this GameState is the hashes of all bottles in the gamestate in some standard order
        // To accomplish this, we first put the bottles of this state into standard order
        let mut std_order_bottles = self.bottles.clone();
        std_order_bottles.sort();

        // we then hash bottles in said order
        for bottle in std_order_bottles {
            bottle.hash(state);
        }
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> TryFrom<&[Bottle<B_MAX_CAP>]>
    for GameState<MAX_BCOUNT, B_MAX_CAP>
{
    type Error = ();
    /// This will only fail if the number of [Bottle]s in the provided `value` exceeds the desired `B_MAX_CAP`.
    fn try_from(value: &[Bottle<B_MAX_CAP>]) -> Result<Self, Self::Error> {
        Ok(GameState {
            bottles: Vec::from_slice(value)?
        })
    }
}
impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> From<[Bottle<B_MAX_CAP>; MAX_BCOUNT]>
    for GameState<MAX_BCOUNT, B_MAX_CAP>
{
    fn from(value: [Bottle<B_MAX_CAP>; MAX_BCOUNT]) -> Self {
        GameState {
            bottles: Vec::from_slice(&value).unwrap()
        }
    }
}
