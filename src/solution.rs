//! Implementation of a [Solution]

use std::{
    collections::{HashMap, VecDeque},
    ops::Deref
};

use bimap::BiHashMap;

use crate::gamestate::{GameState, Pour, PourError, SolvableGameState};

mod demystify;
pub use demystify::{try_demystify_next_step, DemystifyNextStepStats};

mod auto_demystify;
pub use auto_demystify::{auto_demystify, AutoDemystificationResult};

/// Represents the state of a solution finding algorithm
struct SolutionState<GamestateT: SolvableGameState> {
    pub all_gamestates: BiHashMap<usize, GamestateT>,
    pub tried_gamestates: HashMap<usize, (usize, Pour)>,
    pub gamestates_to_try: VecDeque<(u8, usize)>
}

impl<GamestateT: SolvableGameState> SolutionState<GamestateT> {
    /// Create a new SolutionState from a given [SolvableGameState]
    /// that is ready to be passed into [Solution::find_solving_pours]
    pub fn new(gamestate_to_solve: &GamestateT) -> SolutionState<GamestateT> {
        // only generate GameStates once and reference them from here by index.
        // GameStateIdx is an alias to some type we can use to uniquely index into this HashMap;
        // We use an alias to make it more clear what we're doing
        type GameStateIdx = usize;
        let mut all_gamestates: BiHashMap<GameStateIdx, GamestateT> = BiHashMap::new();
        all_gamestates.insert(0, gamestate_to_solve.clone()); // index is 0

        //map from gamestate to (source_gamestate, pour_for_source_gamestate)
        //this allows us to track gamestates
        let tried_gamestates: HashMap<GameStateIdx, (GameStateIdx, Pour)> = HashMap::new();
        let mut gamestates_to_try: VecDeque<(u8, GameStateIdx)> = VecDeque::new();
        gamestates_to_try.push_back((0, 0)); // add our starting gamestate

        Self {
            all_gamestates,
            tried_gamestates,
            gamestates_to_try
        }
    }
}

/// A series of [Pour]s that, when applied to some [SolvableGameState]
/// in sequence, results in a GameState that is finished.
///
/// Although represented as Pours, each Pour is guaranteed
/// to be valid for the result of the previous Pour (except the first Pour, which
/// is instead guaranteed to be valid for the provided `base_gamestate`)
pub struct Solution<T>
where
    T: Deref,
    <T as Deref>::Target: SolvableGameState
{
    base_gamestate: T,
    pours: VecDeque<Pour>
}

impl<T> Solution<T>
where
    T: Deref,
    <T as Deref>::Target: SolvableGameState
{
    /// Try to find and return the shortest possible Solution to the given GameState. If no solution can be found, return `None`.
    ///
    /// Will search for solutions with as many as `max_depth` pours. If no solution is found within that many pours, returns `None`.
    /// Setting `max_depth` to 0 disables the limit; will search for possible solutions that are arbitrarily long until a valid solution is found,
    /// all possible solutions have been checked without a valid solution, or the program panics due to the computer being out of memory.
    pub fn try_new(base_gamestate: T, max_depth: u8) -> Option<Self> {
        let mut solution_state = SolutionState::new(&base_gamestate as &T::Target);

        Self::find_solving_pours(&mut solution_state, max_depth, true).map(|pours| Self {
            base_gamestate,
            pours
        })
    }

    /// Try to create a [Solution] given a base [SolvableGameState] to solve and some iterable of [Pour]s to apply in order
    pub fn try_from_parts<U: IntoIterator<Item = Pour>>(
        base_gamestate: T,
        pours: U
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

    /// For the given SolutionState, generate possible pours and
    /// identify a list of pours that, when applied in order, leads to a solution.
    /// Return value is the list of pours that led to said solution, if one could be found
    ///
    /// `max_depth` specifies how many layers deep to search. When zero, there is effectively no limit to how deep to search.
    ///
    /// The `cutoff` setting only applies when `max_depth` is nonzero (i.e. there is a depth limit set).
    /// When `cutoff` is true, the function will stop generating gamestates that would go beyond `max_depth`.
    /// When `cutoff` is false, the function will generate gamestates that are one layer beyond `max_depth`, but won't evaluate
    /// them; instead returning after it has evaluated the final layer.
    fn find_solving_pours(
        state: &mut SolutionState<T::Target>,
        max_depth: u8,
        cutoff: bool
    ) -> Option<VecDeque<Pour>> {
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
                return None;
            }
            if gamestate_to_try.is_solved() {
                let mut returnable = VecDeque::new();
                let mut gs_idx = gamestate_to_try_idx;
                loop {
                    if let Some((source_gs_idx, pour)) = state.tried_gamestates.remove(&gs_idx) {
                        returnable.push_front(pour);
                        gs_idx = source_gs_idx;
                    } else {
                        //return our solution
                        return Some(returnable);
                    }
                }
            }

            // only explore deeper than this if the max_depth limit is
            // disabled or we aren't yet at the max_depth
            if layer_idx < max_depth || max_depth == 0 || !cutoff {
                // the following could theoretically be done in one loop, but borrow rules prevent this.
                // instead of just cloning, we first create a vector of all new entries that will go into all_gamestates
                let mut new_pours: Vec<(T::Target, Pour)> = Vec::new();
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
        None
    }

    pub fn get_base_gamestate(&self) -> &T::Target {
        &self.base_gamestate
    }

    pub fn get_pours(&self) -> &VecDeque<Pour> {
        &self.pours
    }

    pub fn take_pours(self) -> VecDeque<Pour> {
        self.pours
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
