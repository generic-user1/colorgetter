//! Implementation of [PseudoPartialGameState]
use crate::{
    bottle::{Bottle, BottleSampleResult},
    colored_water::PartialColoredWaterUnit
};

use super::{KnownGameState, PartialGameState};

/// A [KnownGameState] that can be viewed as a [PartialGameState] for testing purposes
pub struct PseudoPartialGameState<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    /// The actual gamestate
    actual: KnownGameState<MAX_BCOUNT, B_MAX_CAP>,

    /// The gamestate viewed as partial
    as_partial: PartialGameState<MAX_BCOUNT, B_MAX_CAP>
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>
    PseudoPartialGameState<MAX_BCOUNT, B_MAX_CAP>
{
    /// Create a new PseudoPartialGameState by marking all non-top units in the provided [KnownGameState]
    /// as unknown
    pub fn new(gs: KnownGameState<MAX_BCOUNT, B_MAX_CAP>) -> Self {
        let mut as_partial: PartialGameState<_, _> = gs.clone().into();
        for bottle in as_partial.bottles.iter_mut() {
            if let Some(top_content_idx) = bottle.get_top_content_idx() {
                for c_idx in 0..top_content_idx {
                    bottle
                        .try_set_color(c_idx, Some(PartialColoredWaterUnit::UnknownColor))
                        .expect("setting a color to unknown failed when it shouldn't");
                }
            }
        }

        //double-check that everything matches
        assert!(states_match(&gs, &as_partial));

        Self {
            actual: gs,
            as_partial
        }
    }

    /// Create a new PseudoPartialGameState from a given [KnownGameState] and [PartialGameState]
    ///
    /// This will fail if `known` doesn't match `partial`; that is, if you couldn't get
    /// `known` from `partial` by purely replacing unknown color units in `partial` with known colors.
    pub fn try_from_parts(
        known: KnownGameState<MAX_BCOUNT, B_MAX_CAP>,
        as_partial: PartialGameState<MAX_BCOUNT, B_MAX_CAP>
    ) -> Result<Self, PseudoPartialGameStateError> {
        if states_match(&known, &as_partial) {
            Ok(Self {
                actual: known,
                as_partial
            })
        } else {
            Err(PseudoPartialGameStateError::StatesDoNotMatch)
        }
    }

    /// Borrow the [KnownGameState] view of this PseudoPartialGameState
    pub fn known(&self) -> &KnownGameState<MAX_BCOUNT, B_MAX_CAP> {
        &self.actual
    }

    /// Borrow the [PartialGameState] view of this PseudoPartialGameState
    pub fn partial(&self) -> &PartialGameState<MAX_BCOUNT, B_MAX_CAP> {
        &self.as_partial
    }

    /// Take both the [KnownGameState] and [PartialGameState] views of this PseudoPartialGameState
    pub fn take(
        self
    ) -> (
        KnownGameState<MAX_BCOUNT, B_MAX_CAP>,
        PartialGameState<MAX_BCOUNT, B_MAX_CAP>
    ) {
        (self.actual, self.as_partial)
    }
}

/// Reasons why creating a [PseudoPartialGameState] may fail
#[derive(Debug)]
pub enum PseudoPartialGameStateError {
    /// The provided [PartialGameState] and [KnownGameState] do not match each other
    StatesDoNotMatch
}

/// Compares a [PartialGameState] `partial` with a [KnownGameState] `known` and
/// returns `true` when the two states match.
///
/// The two states are said to match if you could get `known` purely by replacing
/// the unknown units in `partial` with known colors.
fn states_match<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    known: &KnownGameState<MAX_BCOUNT, B_MAX_CAP>,
    partial: &PartialGameState<MAX_BCOUNT, B_MAX_CAP>
) -> bool {
    //ensure the number of bottles match
    if known.bottles.len() != partial.bottles.len() {
        return false;
    }

    //check each bottle individually
    for (known_bottle, partial_bottle) in known.bottles.iter().zip(partial.bottles.iter()) {
        let cap = known_bottle.capacity();
        if cap != partial_bottle.capacity() {
            return false;
        }
        for c_idx in 0..cap {
            let known_color = known_bottle.sample_at(c_idx);
            let partial_color = partial_bottle.sample_at(c_idx);
            //colors match if they're exactly equal or if known_color is known and partial_color is unknown
            if let BottleSampleResult::KnownColor(_) = known_color {
                if partial_color == known_color || partial_color == BottleSampleResult::UnknownColor
                {
                    continue;
                } else {
                    return false;
                }
            }
        }
    }
    //if we didn't return false at any point, the states must match
    true
}
