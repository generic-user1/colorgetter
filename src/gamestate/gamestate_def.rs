use super::*;
use crate::bottle::Bottle;
use crossterm::{
    cursor::{MoveDown, MoveLeft, MoveRight},
    style::{ContentStyle, Print, PrintStyledContent, StyledContent},
    QueueableCommand
};
use heapless::Vec;
use std::{hash::Hash, io, usize};

/// The state a particular game is in
///
/// That is, represents what bottles exist and what order they're in.
#[derive(Debug, Clone, Eq)]
pub struct GameState<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    pub bottles: Vec<Bottle<B_MAX_CAP>, MAX_BCOUNT>
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> GameState<MAX_BCOUNT, B_MAX_CAP> {
    /// Queues the display of this GameState for the given `ostream` (typically [std::io::stdout])
    ///
    /// Does not flush; the caller must call flush on `ostream` in order to actually display the GameState.
    pub fn queue_display<T: QueueableCommand>(&self, ostream: &mut T) -> io::Result<()> {
        // TODO: break GameState into multiple lines in some cases

        const FILLED: char = '█';
        const EMPTY: char = '▒';
        const NOTHING: char = ' ';

        // Cursor movement doesn't appear to work right on Windows, so we use regular characters.
        // Using an alternate screen might make cursor movement work though, so this is left as an option for now.
        const USE_CURSOR: bool = false;

        if let Some(row_count) = self.bottles.iter().map(|b| b.get_capacity()).max() {
            for row_index in (0..row_count).rev() {
                for bottle in &self.bottles {
                    if bottle.get_capacity() < (row_index + 1) {
                        // if capacity is less than row index + 1 (row_index is 0-based so we add 1 for 1-based),
                        // this bottle doesn't have the capacity to reach here, so we print nothing
                        if USE_CURSOR {
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
                            FILLED
                        );
                        ostream.queue(PrintStyledContent(styled_content))?;
                    } else {
                        // bottle has capacity to reach here, but there's no content there;
                        // print empty space
                        ostream.queue(Print(EMPTY))?;
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
