use std::fmt::Display;

use super::*;
use crate::bottle::{Bottle, PourOutError};

/// The operation of pouring content from one [Bottle] into another within the same [GameState].
///
/// If a ValidPour exists, it is guaranteed to be valid; that is, [ValidPour::apply] can never fail.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidPour<'a> {
    /// the GameState this ValidPour applies to
    source_gamestate: &'a GameState,
    /// the source bottle's index within the GameState
    source_bottle_index: usize,
    /// the dest bottle's index within the GameState
    dest_bottle_index: usize
}

impl<'a> ValidPour<'a> {
    /// Try to create a new [ValidPour]
    pub fn try_new(
        source_gamestate: &'a GameState,
        source_bottle_index: usize,
        dest_bottle_index: usize
    ) -> Result<Self, PourError> {
        // check if we got the same index twice
        if source_bottle_index == dest_bottle_index {
            return Err(PourError::SameBottle);
        }

        // get references to both bottles
        let source_bottle = source_gamestate
            .bottles
            .get(source_bottle_index)
            .ok_or(PourError::MissingBottle)?;

        let dest_bottle = source_gamestate
            .bottles
            .get(dest_bottle_index)
            .ok_or(PourError::MissingBottle)?;

        // test whether the pour would succeed by copying the two bottles and performing the pour on those copies
        if let Err(e) = source_bottle.test_pour_out(dest_bottle) {
            return Err(e.into());
        }

        // if we reached this point, the pour is valid; construct and return
        Ok(ValidPour {
            source_gamestate,
            source_bottle_index,
            dest_bottle_index
        })
    }

    /// Create a new [GameState] that is the result of applying this ValidPour to the source [GameState]
    pub fn apply(&self) -> GameState {
        let mut new_game_state = self.source_gamestate.clone();

        let split_on_source = self.source_bottle_index < self.dest_bottle_index;

        let (left, right) = new_game_state.bottles.split_at_mut(
            if split_on_source {
                self.source_bottle_index
            } else {
                self.dest_bottle_index
            } + 1
        );
        let left_len = left.len();
        let (source_bottle, dest_bottle) = if split_on_source {
            (
                left.get_mut(self.source_bottle_index).unwrap(),
                right.get_mut(self.dest_bottle_index - left_len).unwrap()
            )
        } else {
            (
                left.get_mut(self.dest_bottle_index).unwrap(),
                right.get_mut(self.source_bottle_index - left_len).unwrap()
            )
        };

        source_bottle
            .try_pour_out(dest_bottle)
            .expect("Applying a Pour resulted in a PourOutError");
        new_game_state
    }

    pub fn get_source_gamestate(&self) -> &GameState {
        self.source_gamestate
    }

    pub fn get_source_index(&self) -> usize {
        self.source_bottle_index
    }
    pub fn get_source(&self) -> &Bottle {
        self.source_gamestate
            .bottles
            .get(self.source_bottle_index)
            .unwrap()
    }

    pub fn get_dest_index(&self) -> usize {
        self.dest_bottle_index
    }
    pub fn get_dest(&self) -> &Bottle {
        self.source_gamestate
            .bottles
            .get(self.dest_bottle_index)
            .unwrap()
    }
}

impl<'a> Display for ValidPour<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ValidPour(from: {}, to: {})",
            self.source_bottle_index, self.dest_bottle_index
        )?;

        Ok(())
    }
}

/// Reasons creating a [ValidPour] may fail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PourError {
    /// A provided bottle index doesn't point to a [Bottle] within the given [GameState];
    /// may be the source, destination, or both.
    MissingBottle,

    /// The two provided bottle indices are identical.
    SameBottle,

    /// This pour, if carried out, would lead to a [PourOutError] of the provided type.
    InvalidPour(PourOutError)
}

impl From<PourOutError> for PourError {
    fn from(value: PourOutError) -> Self {
        PourError::InvalidPour(value)
    }
}

/// An iterator over all valid [ValidPour]s you could apply to a given [GameState]
pub struct ValidPourIter<'a> {
    source_gamestate: &'a GameState,
    current_from_index: usize,
    current_to_index: usize
}
impl<'a> ValidPourIter<'a> {
    pub fn new(source_gamestate: &'a GameState) -> Self {
        Self {
            source_gamestate,
            current_from_index: 0,
            current_to_index: 0
        }
    }
}
impl<'a> Iterator for ValidPourIter<'a> {
    type Item = ValidPour<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        for from_index in self.current_from_index..self.source_gamestate.bottles.len() {
            for to_index in self.current_to_index..self.source_gamestate.bottles.len() {
                if let Ok(pour) = ValidPour::try_new(self.source_gamestate, from_index, to_index) {
                    self.current_from_index = from_index;
                    // our "current" to_index needs to be the next index we'll use, so the index we just used plus 1
                    // note that if this exceeds the bounds of our current_to_index..bottles.len() range, that range will be
                    // empty; calling next will therefore cause from_index to increment instead
                    self.current_to_index = to_index.saturating_add(1);
                    return Some(pour);
                }
            }
            //reset to_index just before going through loop with new from_index
            self.current_to_index = 0;
        }
        None
    }
}
