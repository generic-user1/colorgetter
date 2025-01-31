//! Implementation of a [Solution]

use std::{
    collections::{HashMap, VecDeque},
    io::{self, Write},
    time::Instant
};

use bimap::BiHashMap;

use crate::gamestate::{GameState, Pour};

/// A series of [Pour]s that, when applied to some [GameState]
/// in sequence, results in a GameState that is finished.
///
/// Although represented as Pours, each Pour is guaranteed
/// to be valid for the result of the previous Pour (except the first Pour, which
/// is instead guaranteed to be valid for the provided `base_gamestate`)
pub struct Solution<'a, const N: usize> {
    base_gamestate: &'a GameState<N>,
    pours: VecDeque<Pour>
}

impl<'a, const N: usize> Solution<'a, N> {
    /// Try to find and return a Solution to the given GameState. If no solution can be found, return `None`.
    pub fn try_new(base_gamestate: &'a GameState<N>) -> Option<Self> {
        Self::find_solving_pours(base_gamestate).map(|pours| Self {
            base_gamestate,
            pours
        })
    }

    /// Given a GameState, generate possible pours and identify a list of pours that,
    /// when applied in order, leads to a solution.
    ///
    /// Returns None if there are no solutions.
    fn find_solving_pours(gamestate_to_solve: &GameState<N>) -> Option<VecDeque<Pour>> {
        const PRINT_TIMING_METRICS: bool = true;

        // only generate GameStates once and reference them from here by index.
        // GameStateIdx is an alias to some type we can use to uniquely index into this HashMap;
        // We use an alias to make it more clear what we're doing
        type GameStateIdx = usize;
        let mut all_gamestates: BiHashMap<GameStateIdx, GameState<N>> = BiHashMap::new();
        all_gamestates.insert(0, gamestate_to_solve.clone()); // index is 0

        //map from gamestate to (source_gamestate, pour_for_source_gamestate)
        //this allows us to track gamestates
        let mut tried_gamestates: HashMap<GameStateIdx, (GameStateIdx, Pour)> = HashMap::new();
        let mut gamestates_to_try: VecDeque<(u8, GameStateIdx)> = VecDeque::new();
        gamestates_to_try.push_back((0, 0)); // add our starting gamestate

        let overall_start_time = Instant::now();
        let mut layer_start_time = overall_start_time;
        let mut last_seen_layer_idx: u8 = 0;
        let mut states_within_layer: usize = 0;

        while let Some((layer_idx, gamestate_to_try_idx)) = gamestates_to_try.pop_front() {
            if PRINT_TIMING_METRICS {
                if layer_idx != last_seen_layer_idx {
                    if last_seen_layer_idx != 0 {
                        let layer_end_time = Instant::now();
                        println!(
                            "Done; ({} members processed in {:?}) (overall time: {:?})",
                            states_within_layer + 1,
                            layer_end_time.duration_since(layer_start_time),
                            layer_end_time.duration_since(overall_start_time)
                        );
                        layer_start_time = layer_end_time
                    }
                    print!("Starting layer with index {}... ", layer_idx);
                    io::stdout().flush().unwrap();
                    last_seen_layer_idx = layer_idx;
                    states_within_layer = 0;
                } else {
                    states_within_layer += 1;
                }
            }
            let gamestate_to_try = all_gamestates.get_by_left(&gamestate_to_try_idx).unwrap();
            if gamestate_to_try.is_finished() {
                let layer_end_time = Instant::now();
                if PRINT_TIMING_METRICS {
                    println!(
                        "Found solution! ({} members processed in {:?})",
                        states_within_layer,
                        layer_end_time.duration_since(layer_start_time),
                    );
                    println!(
                        "Overall, evaluated {} members in {:?}",
                        tried_gamestates.len(),
                        layer_end_time.duration_since(overall_start_time)
                    );
                }
                let mut returnable = VecDeque::new();
                let mut gs_idx = gamestate_to_try_idx;
                loop {
                    if let Some((source_gs_idx, pour)) = tried_gamestates.remove(&gs_idx) {
                        returnable.push_front(pour);
                        gs_idx = source_gs_idx;
                    } else {
                        return Some(returnable);
                    }
                }
            }

            // the following could theoretically be done in one loop, but borrow rules prevent this.
            // instead of just cloning, we first create a vector of all new entries that will go into all_gamestates
            let mut new_pours: Vec<(GameState<N>, Pour)> = Vec::new();
            for valid_pour in gamestate_to_try.iter_pours() {
                let new_gs = valid_pour.apply();
                // only check this gamestate if we haven't already checked it
                // this means it isn't in tried_gamestates (we didn't generate it)
                // *and* it isn't the gamestate we started with
                if all_gamestates.get_by_right(&new_gs).is_none() && new_gs != *gamestate_to_solve {
                    new_pours.push((new_gs, valid_pour.into()));
                }
            }

            // second, since we no longer need gamestate_to_try, we're no longer borrowing immutably from all_gamestates;
            // so we are allowed to borrow mutably from it (required to add new gamestates to it)
            for (new_gs, pour) in new_pours {
                let new_gs_idx = all_gamestates.len();
                all_gamestates.insert(new_gs_idx, new_gs);

                tried_gamestates.insert(new_gs_idx, (gamestate_to_try_idx, pour));
                gamestates_to_try.push_back((layer_idx.wrapping_add(1), new_gs_idx));
            }
        }

        // we iterated through all possible pours from this gamestate and found no pours that
        // lead to a solution; it's not possible to win from here
        None
    }

    pub fn get_base_gamestate(&self) -> &GameState<N> {
        self.base_gamestate
    }

    pub fn get_pours(&self) -> &VecDeque<Pour> {
        &self.pours
    }
}
