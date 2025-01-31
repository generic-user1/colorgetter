//! Implementation of a [Solution]

use std::{
    collections::{HashMap, VecDeque},
    io::{self, Write},
    time::Instant
};

use crate::gamestate::{GameState, Pour};

/// A series of [Pour]s that, when applied to some [GameState]
/// in sequence, results in a GameState that is finished.
///
/// Although represented as Pours, each Pour is guaranteed
/// to be valid for the result of the previous Pour (except the first Pour, which
/// is instead guaranteed to be valid for the provided `base_gamestate`)
pub struct Solution<'a> {
    base_gamestate: &'a GameState,
    pours: VecDeque<Pour>
}

impl<'a> Solution<'a> {
    /// Try to find and return a Solution to the given GameState. If no solution can be found, return `None`.
    pub fn try_new(base_gamestate: &'a GameState) -> Option<Self> {
        Self::find_solving_pours(base_gamestate).map(|pours| Self {
            base_gamestate,
            pours
        })
    }

    /// Given a GameState, generate possible pours and identify a list of pours that,
    /// when applied in order, leads to a solution.
    ///
    /// Returns None if there are no solutions.
    fn find_solving_pours(gamestate_to_solve: &GameState) -> Option<VecDeque<Pour>> {
        const PRINT_TIMING_METRICS: bool = true;

        //map from gamestate to (source_gamestate, pour_for_source_gamestate)
        //this allows us to track gamestates
        let mut tried_gamestates: HashMap<GameState, (GameState, Pour)> = HashMap::new();
        let mut gamestates_to_try: VecDeque<(u8, GameState)> = VecDeque::new();
        gamestates_to_try.push_back((0, gamestate_to_solve.clone()));

        let overall_start_time = Instant::now();
        let mut layer_start_time = overall_start_time;
        let mut last_seen_layer_idx: u8 = 0;
        let mut states_within_layer: usize = 0;

        while let Some((layer_idx, gamestate_to_try)) = gamestates_to_try.pop_front() {
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
                let mut gs = gamestate_to_try;
                loop {
                    if let Some((source_gs, pour)) = tried_gamestates.get(&gs) {
                        returnable.push_front(pour.clone());
                        gs = source_gs.clone();
                    } else {
                        return Some(returnable);
                    }
                }
            }

            for valid_pour in gamestate_to_try.iter_pours() {
                let new_gs = valid_pour.apply();
                // only check this gamestate if we haven't already checked it
                // this means it isn't in tried_gamestates (we didn't generate it)
                // *and* it isn't the gamestate we started with
                if !tried_gamestates.contains_key(&new_gs) && new_gs != *gamestate_to_solve {
                    tried_gamestates.insert(
                        new_gs.clone(),
                        (gamestate_to_try.clone(), valid_pour.into())
                    );
                    gamestates_to_try.push_back((layer_idx.wrapping_add(1), new_gs));
                }
            }
        }

        // we iterated through all possible pours from this gamestate and found no pours that
        // lead to a solution; it's not possible to win from here
        None
    }

    pub fn get_base_gamestate(&self) -> &GameState {
        self.base_gamestate
    }

    pub fn get_pours(&self) -> &VecDeque<Pour> {
        &self.pours
    }
}
