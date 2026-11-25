//! Implementation of a [Solution]

use std::collections::VecDeque;

use crate::gamestate::{PartialGameState, Pour, PourError, SolvableGameState};

mod solution_state;
use solution_state::SolutionState;

mod demystify;
pub use demystify::{try_demystify_next_step, DemystifyNextStepStats};

mod auto_demystify;
pub use auto_demystify::{auto_demystify, AutoDemystificationResult};

/// A series of [Pour]s that, when applied to some [SolvableGameState]
/// in sequence, results in a GameState that is finished.
///
/// Although represented as Pours, each Pour is guaranteed
/// to be valid for the result of the previous Pour (except the first Pour, which
/// is instead guaranteed to be valid for the provided `base_gamestate`)
#[derive(Debug, Clone)]
pub struct Solution<'a, GamestateT: SolvableGameState> {
    base_gamestate: &'a GamestateT,
    pours: VecDeque<Pour>
}

impl<'a, GamestateT: SolvableGameState> Solution<'a, GamestateT> {
    /// Try to find and return the shortest possible Solution to the given GameState. If no solution can be found, return `None`.
    ///
    /// Will search for solutions with as many as `max_depth` pours. If no solution is found within that many pours, returns `None`.
    /// Setting `max_depth` to 0 disables the limit; will search for possible solutions that are arbitrarily long until a valid solution is found,
    /// all possible solutions have been checked without a valid solution, or the program panics due to the computer being out of memory.
    pub fn try_new(base_gamestate: &'a GamestateT, max_depth: u8) -> Option<Self> {
        let mut solution_state = SolutionState::new(base_gamestate);

        find_solving_pours(&mut solution_state, is_solved, max_depth, true, 1);
        solution_state
            .take_solving_pour_sequence(0)
            .map(|pours| Self {
                base_gamestate,
                pours
            })
    }

    /// Try to create a [Solution] given a base [SolvableGameState] to solve and some iterable of [Pour]s to apply in order
    pub fn try_from_parts<T: IntoIterator<Item = Pour>>(
        base_gamestate: &'a GamestateT,
        pours: T
    ) -> Result<Self, SolutionFromPartsError> {
        let mut working_gs = base_gamestate.clone();
        let mut owned_pours: VecDeque<Pour> = VecDeque::new();
        for pour in pours {
            let as_valid = pour.try_into_valid(&working_gs)?;
            working_gs = as_valid.apply();
            owned_pours.push_back(pour);
        }
        if working_gs.is_solved() {
            Ok(Self {
                base_gamestate,
                pours: owned_pours
            })
        } else {
            Err(SolutionFromPartsError::DoesNotFinish)
        }
    }

    pub fn get_base_gamestate(&self) -> &GamestateT {
        self.base_gamestate
    }

    pub fn get_pours(&self) -> &VecDeque<Pour> {
        &self.pours
    }

    pub fn take_pours(self) -> VecDeque<Pour> {
        self.pours
    }
}

/// Try to find and return as many Solutions as possible for the given [PartialGameState]
///
/// Will search for solutions with as many as `max_depth` pours. Setting `max_depth` to 0 disables the limit;
/// will search for possible solutions that are arbitrarily long.
///
/// Will first search for as many as `max_ordered` solutions treating the game state as ordered. If
/// `max_ordered` solutions are found, will instead treat the game state as orderless and search for up to
/// `max_orderless` solutions. This is to enable conditional ordered/orderless searches based on the number of
/// solutions possible - the idea being that if there are few solutions, ordered is preferable (to enumerate every possible solution),
/// but if there are many solutions, orderless is preferable (to get a wider range of possible solutions).
/// Setting `max_ordered` or `max_orderless` to 0 disables the respective limits - if `max_ordered` is zero, will never transition to
/// orderless searching, and if `max_orderless` is 0, will not stop searching in orderless mode until all possible solutions are found.
///
/// *Note*: due to some important optimizations, it is currently not possible to find multiple solutions to a
/// [KnownGameState](crate::gamestate::KnownGameState). Therefore, this method only accepts [PartialGameState]s.
pub fn find_many_solutions<'a, const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    base_gamestate: &'a PartialGameState<MAX_BCOUNT, B_MAX_CAP, false>,
    max_depth: u8,
    max_ordered: usize,
    max_orderless: usize
) -> Vec<Solution<'a, PartialGameState<MAX_BCOUNT, B_MAX_CAP, false>>> {
    let mut solution_state = SolutionState::new(base_gamestate);

    find_solving_pours(&mut solution_state, is_solved, max_depth, true, max_ordered);
    let pour_sequences = if solution_state.all_solving_pour_sequences().len() == max_ordered {
        //try again with an orderless state
        let as_orderless: PartialGameState<MAX_BCOUNT, B_MAX_CAP, true> =
            base_gamestate.clone().into();
        let mut solution_state = SolutionState::new(&as_orderless);
        find_solving_pours(
            &mut solution_state,
            is_solved,
            max_depth,
            true,
            max_orderless
        );
        solution_state.all_solving_pour_sequences()
    } else {
        //use solution_state unchanged; since we're already under the limit, an orderless run won't help us
        //get any more solutions
        solution_state.all_solving_pour_sequences()
    };
    pour_sequences
        .into_iter()
        .map(|pours| Solution {
            base_gamestate,
            pours
        })
        .collect()
}

/// For some [SolvableGameState] `base_gamestate`, try to find a [Solution] that, instead of leading to a solved state, leads to a specific `target` state.
pub fn find_path_to_specific<'a, GamestateT: SolvableGameState>(
    base_gamestate: &'a GamestateT,
    target: &GamestateT,
    max_depth: u8
) -> Option<Solution<'a, GamestateT>> {
    let mut solution_state = SolutionState::new(base_gamestate);

    let predicate = |gs: &GamestateT| gs == target;

    find_solving_pours(&mut solution_state, predicate, max_depth, true, 1);
    solution_state
        .take_solving_pour_sequence(0)
        .map(|pours| Solution {
            base_gamestate,
            pours
        })
}

/// Basic predicate for [find_solving_pours] that simply returns the result of the gamestate's [SolvableGameState::is_solved]
fn is_solved<T: SolvableGameState>(gs: &T) -> bool {
    gs.is_solved()
}

/// For the given SolutionState, generate possible pours and
/// identify a list of pours that, when applied in order, leads to a solution.
///
/// Winning game state indexes are added to the given `state` and can be converted into sequences of pours
/// by calling the `get_solving_pour_sequences` method of the SolutionState.
///
/// `predicate` is some Fn that accepts some gamestate and returns whether it is solved as a boolean. Through this, you can alter
/// what it means to be 'solved'. Most of the time, you will want to use `is_solved`, which simply uses the gamestate's [SolvableGameState::is_solved]
/// to make this determination.
///
/// `max_depth` specifies how many layers deep to search. When zero, there is effectively no limit to how deep to search.
///
/// The `cutoff` setting only applies when `max_depth` is nonzero (i.e. there is a depth limit set).
/// When `cutoff` is true, the function will stop generating gamestates that would go beyond `max_depth`.
/// When `cutoff` is false, the function will generate gamestates that are one layer beyond `max_depth`, but won't evaluate
/// them; instead returning after it has evaluated the final layer.
///
/// `max_solution_count` is the number of solutions found after which to stop searching for more. If this is zero,
/// there is effectively no limit.
fn find_solving_pours<T: SolvableGameState>(
    state: &mut SolutionState<T>,
    predicate: impl Fn(&T) -> bool,
    max_depth: u8,
    cutoff: bool,
    max_solution_count: usize
) {
    while let Some((layer_idx, gamestate_to_try_idx)) = state.gamestates_to_try.pop_front() {
        let gamestate_to_try = state
            .all_gamestates
            .get_by_left(&gamestate_to_try_idx)
            .unwrap();
        if layer_idx > max_depth && max_depth > 0 {
            // put the state we just popped off back onto gamestates_to_try so that someone looking
            // at our solutionstate can see what we were about to do
            state
                .gamestates_to_try
                .push_front((layer_idx, gamestate_to_try_idx));
            return;
        }

        let gs_is_solved = predicate(gamestate_to_try);

        if gs_is_solved {
            state.finished_gamestates.push(gamestate_to_try_idx);
            if max_solution_count > 0 && state.finished_gamestates.len() >= max_solution_count {
                return;
            }
            //don't evaluate children of finished gamestates
            continue;
        }

        // only explore deeper than this if the max_depth limit is
        // disabled or we aren't yet at the max_depth
        if layer_idx < max_depth || max_depth == 0 || !cutoff {
            // the following could theoretically be done in one loop, but borrow rules prevent this.
            // instead of just cloning, we first create a vector of all new entries that will go into all_gamestates
            let mut new_pours: Vec<(T, Pour)> = Vec::new();
            for valid_pour in gamestate_to_try.iter_pours() {
                let new_gs = valid_pour.apply();
                // only check this gamestate if we haven't already checked it
                // this means it isn't in tried_gamestates (we didn't generate it)
                // *and* it isn't the gamestate we started with
                if state.all_gamestates.get_by_right(&new_gs).is_none() {
                    new_pours.push((new_gs, valid_pour.into()));
                }
            }

            // since some of the valid pours may be functional duplicates, we need to dedup here
            new_pours.sort_by(|a, b| a.0.cmp(&b.0));
            new_pours.dedup_by(|a, b| a.0.eq(&b.0));

            // second, since we no longer need gamestate_to_try, we're no longer borrowing immutably from all_gamestates;
            // so we are allowed to borrow mutably from it (required to add new gamestates to it)
            for (new_gs, pour) in new_pours.into_iter() {
                let new_gs_idx = state.all_gamestates.len();
                state.all_gamestates.insert(new_gs_idx, new_gs);

                state
                    .tried_gamestates
                    .insert(new_gs_idx, (gamestate_to_try_idx, pour));
                // we use saturating add for the new layer_idx to ensure that if we reach layer 255,
                // items on layer 256 aren't said to be on layer 0; if the layer idx is going to
                // be incorrect, we'd prefer it to at least never go down
                state
                    .gamestates_to_try
                    .push_back((layer_idx.saturating_add(1), new_gs_idx));
            }
        }
    }
}

/// Reasons running [Solution::try_from_parts] may fail
#[derive(Debug)]
pub enum SolutionFromPartsError {
    /// Encountered a [PourError] while checking that [Pour]s are valid
    PourError(PourError),

    /// The sequence of [Pour]s is valid for the given [SolvableGameState], but
    /// does not result in a finished [SolvableGameState]
    DoesNotFinish
}

impl From<PourError> for SolutionFromPartsError {
    fn from(value: PourError) -> Self {
        SolutionFromPartsError::PourError(value)
    }
}
