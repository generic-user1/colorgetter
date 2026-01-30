//! Implementation of [SolutionState]

use std::collections::{HashMap, VecDeque};

use bimap::BiHashMap;

use crate::gamestate::{Pour, SolvableGameState};

/// Represents the state of a solution finding algorithm
pub(super) struct SolutionState<GamestateT: SolvableGameState> {
    /// Bijective mapping of numeric IDs (left) to GameStates (right).
    ///
    /// Exists for two reasons:
    /// - It allows us to pass around GameStates by ID; IDs are only 8 bytes,
    ///   and GameStates are likely much larger (their exact size depends on t
    ///   heir values of `MAX_BCOUNT` and `B_MAX_CAP`)
    /// - It allows us to easily check whether a GameState has been seen before so we can
    ///   skip redundant checks on GameStates that can be reached through multiple paths
    pub all_gamestates: BiHashMap<usize, GamestateT>,

    /// Mapping of numeric GameState IDs (left) to 2-tuples `(source_id, pour)`.
    ///
    /// This is used to determine how we can get to some GameState. The GameState
    /// on the left can be reached by applying the pour on the right to the GameState ID on the right.
    pub tried_gamestates: HashMap<usize, (usize, Pour)>,

    /// Sequence of 2-tuples `(layer_idx, gamestate_id)`
    ///
    /// This is the sequence of GameStates that we want to check but haven't checked yet.
    /// It is a [VecDeque] rather than a [Vec] because we want FIFO ordering.
    pub gamestates_to_try: VecDeque<(u8, usize)>,

    /// Sequence of numeric GameState IDs that are known to be solved
    /// (i.e. GameStates where [SolvableGameState::is_solved] returns `true`)
    ///
    /// The order of GameState IDs matches the order they were found in. Since
    /// we use breadth-first search, this means that GameState IDs appearing earlier
    /// in the sequence take as many or fewer Pours to get to from the initial state.
    pub finished_gamestates: Vec<usize>
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
            gamestates_to_try,
            finished_gamestates: Vec::new()
        }
    }

    /// Generate a [Vec] of all the solving pour sequences found
    pub fn all_solving_pour_sequences(&self) -> Vec<VecDeque<Pour>> {
        let mut output = Vec::with_capacity(self.finished_gamestates.len());

        for gamestate_idx in self.finished_gamestates.iter() {
            let mut this_sequence = VecDeque::new();
            let mut gs_idx = *gamestate_idx;
            loop {
                if let Some((source_gs_idx, pour)) = self.tried_gamestates.get(&gs_idx) {
                    this_sequence.push_front(pour.clone());
                    gs_idx = *source_gs_idx;
                } else {
                    output.push(this_sequence);
                    break;
                }
            }
        }
        output
    }

    /// Generate a single solving pour sequence without copying, consuming this SolutionState
    ///
    /// The `idx` parameter should point to a valid index into this SolutionState's `finished_gamestates`.
    /// If the given `idx` does not point to some value in `finished_gamestates`, [None] will be returned.
    pub fn take_solving_pour_sequence(mut self, idx: usize) -> Option<VecDeque<Pour>> {
        if let Some(mut gs_idx) = self.finished_gamestates.into_iter().nth(idx) {
            let mut output = VecDeque::new();
            loop {
                if let Some((source_gs_idx, pour)) = self.tried_gamestates.remove(&gs_idx) {
                    output.push_front(pour);
                    gs_idx = source_gs_idx;
                } else {
                    return Some(output);
                }
            }
        } else {
            None
        }
    }
}
