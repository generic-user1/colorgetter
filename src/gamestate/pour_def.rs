use crate::bottle::{Bottle, PourOutError};
use crate::gamestate::GameState;

/// The operation of pouring content from one [Bottle] into another within the same [GameState].
///
/// If a Pour exists, it is guaranteed to be a valid move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pour<'a> {
    /// the GameState this Pour applies to
    source_gamestate: &'a GameState,
    /// the source bottle's index within the GameState
    source_bottle_index: usize,
    /// the dest bottle's index within the GameState
    dest_bottle_index: usize
}

impl<'a> Pour<'a> {
    /// Try to create a new [Pour]
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
        if let Err(e) = source_bottle.clone().try_pour_out(&mut dest_bottle.clone()) {
            return Err(e.into());
        }

        // if we reached this point, the pour is valid; construct and return
        Ok(Pour {
            source_gamestate,
            source_bottle_index,
            dest_bottle_index
        })
    }

    /// Create a new [GameState] that is the result of applying this Pour to the source [GameState]
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

/// Reasons creating a [Pour] may fail
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
