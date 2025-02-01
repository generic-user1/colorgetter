//! Implementation of a [Solution]

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
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
    /// Try to find and return the shortest possible Solution to the given GameState. If no solution can be found, return `None`.
    ///
    /// May take a relatively long time and a lot of memory, but will always return a Solution
    /// with the fewest possible pours. If you want a solution faster and don't care if it uses more pours
    /// than necessary, see [Solution::try_new_fast_find]
    pub fn try_new_shortest(base_gamestate: &'a GameState<N>) -> Option<Self> {
        Self::find_solving_pours_breadth_first(base_gamestate).map(|pours| Self {
            base_gamestate,
            pours
        })
    }

    /// Try to find and return any Solution to the given GameState. If no solution can be found, return `None`.
    ///
    /// Solution returned may have many more Pours than necessary, but will almost always
    /// take relatively little time and memory to compute. If you want a short solution and don't care
    /// if it takes more time and memory, see [Solution::try_new_shortest]
    pub fn try_new_fast_find(base_gamestate: &'a GameState<N>, max_depth: u8) -> Option<Self> {
        Self::find_solving_pours_depth_first(base_gamestate, max_depth).map(|pours| Self {
            base_gamestate,
            pours
        })
    }

    /// Given a GameState, generate possible pours and identify a list of pours that,
    /// when applied in order, leads to a solution.
    ///
    /// Returns None if there are no solutions.
    fn find_solving_pours_breadth_first(
        gamestate_to_solve: &GameState<N>
    ) -> Option<VecDeque<Pour>> {
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
                if PRINT_TIMING_METRICS {
                    let layer_end_time = Instant::now();
                    println!(
                        "Found solution! ({} members processed in {:?}) ({} members in layer)",
                        states_within_layer,
                        layer_end_time.duration_since(layer_start_time),
                        states_within_layer
                            + gamestates_to_try
                                .drain(..)
                                .map(|(l, _)| {
                                    if l == layer_idx {
                                        1
                                    } else {
                                        0
                                    }
                                })
                                .sum::<usize>()
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
                if all_gamestates.get_by_right(&new_gs).is_none() {
                    new_pours.push((new_gs, valid_pour.into()));
                }
            }

            // second, since we no longer need gamestate_to_try, we're no longer borrowing immutably from all_gamestates;
            // so we are allowed to borrow mutably from it (required to add new gamestates to it)
            for (new_gs, pour) in new_pours.into_iter() {
                let new_gs_idx = all_gamestates.len();
                all_gamestates.insert(new_gs_idx, new_gs);

                tried_gamestates.insert(new_gs_idx, (gamestate_to_try_idx, pour));
                gamestates_to_try.push_back((layer_idx.wrapping_add(1), new_gs_idx));
            }
        }

        // we iterated through all possible pours from this gamestate and found no pours that
        // lead to a solution; it's not possible to win from here
        if PRINT_TIMING_METRICS {
            let end_time = Instant::now();
            println!(
                "Done; ({} members processed in {:?})",
                states_within_layer,
                end_time.duration_since(layer_start_time),
            );
            println!(
                "Overall, evaluated {} members in {:?}, but did not find a solution",
                tried_gamestates.len(),
                end_time.duration_since(overall_start_time)
            );
        }
        None
    }

    fn find_solving_pours_depth_first(
        gamestate_to_solve: &GameState<N>,
        max_depth: u8
    ) -> Option<VecDeque<Pour>> {
        // only generate GameStates once and reference them from here by index.
        // GameStateIdx is an alias to some type we can use to uniquely index into this HashMap;
        // We use an alias to make it more clear what we're doing
        type GameStateIdx = usize;
        let mut all_gamestates: BiHashMap<GameStateIdx, GameState<N>> = BiHashMap::new();
        all_gamestates.insert(0, gamestate_to_solve.clone()); // index is 0

        // for all gamestates, track which layer we found them at
        let mut all_gamestate_layers: HashMap<GameStateIdx, u8> = HashMap::new();
        all_gamestate_layers.insert(0, 0);

        let mut visited_at_each_layer_count: BTreeMap<u8, usize> = BTreeMap::new();

        //map from gamestate to (source_gamestate, pour_for_source_gamestate)
        //this allows us to track gamestates
        let mut tried_gamestates: HashMap<GameStateIdx, (GameStateIdx, Pour)> = HashMap::new();
        let mut gamestates_to_try: Vec<GameStateIdx> = Vec::new();
        gamestates_to_try.push(0); // add our starting gamestate

        while let Some(gamestate_to_try_idx) = gamestates_to_try.pop() {
            // we need both the gamestate we're testing and the layer we found it on
            let gamestate_to_try = all_gamestates.get_by_left(&gamestate_to_try_idx).unwrap();
            let layer_idx = *all_gamestate_layers.get(&gamestate_to_try_idx).unwrap();

            if gamestate_to_try.is_finished() {
                println!(
                    "Found solution on layer {} after checking {} gamestates",
                    layer_idx,
                    tried_gamestates.len()
                );
                let mut returnable = VecDeque::new();
                let mut gs_idx = gamestate_to_try_idx;
                loop {
                    if let Some((source_gs_idx, pour)) = tried_gamestates.remove(&gs_idx) {
                        returnable.push_front(pour);
                        gs_idx = source_gs_idx;
                    } else {
                        println!("visited counts: {:?}", visited_at_each_layer_count);
                        return Some(returnable);
                    }
                }
            }

            let current_visited_for_layer =
                *visited_at_each_layer_count.get(&layer_idx).unwrap_or(&0);
            visited_at_each_layer_count.insert(layer_idx, current_visited_for_layer + 1);

            // only explore deeper than this if the max_depth limit is
            // disabled or we aren't yet at the max_depth
            if layer_idx < max_depth || max_depth == 0 {
                // the following could theoretically be done in one loop, but borrow rules prevent this.
                // instead of just cloning, we first create a vector of data we'll need.
                // vector's entries are `(game_state, pour_to_get_to_new_gamestate, game_state_idx)`
                // note that game_state_idx is optional; this gamestate may never have been seen before
                let mut new_pours: Vec<(GameState<N>, Pour, Option<GameStateIdx>)> = Vec::new();
                let new_layer_idx = layer_idx.wrapping_add(1);
                for valid_pour in gamestate_to_try.iter_pours() {
                    let new_gs = valid_pour.apply();

                    // what we place into the new_pours vec depends on whether we have seen this gamestate before,
                    // and if we have, what layer we saw it at
                    match all_gamestates.get_by_right(&new_gs) {
                        None => {
                            // never before seen, has no gs_idx
                            new_pours.push((new_gs, valid_pour.into(), None));
                        }
                        Some(existing_gs_idx) => {
                            // has been seen before; only add to vec if it was seen on a lower layer;
                            // we don't want to check this gamestate again if we've already seen it on a higher layer
                            let existing_gs_layer_idx =
                                *all_gamestate_layers.get(existing_gs_idx).unwrap();

                            if existing_gs_layer_idx > new_layer_idx {
                                new_pours.push((new_gs, valid_pour.into(), Some(*existing_gs_idx)));
                            }
                        }
                    }
                }

                // second, since we no longer need gamestate_to_try, we're no longer borrowing immutably from all_gamestates;
                // so we are allowed to borrow mutably from it (required to add new gamestates to it)
                for (new_gs, pour, existing_gs_idx) in new_pours {
                    let new_gs_idx = if let Some(existing_gs_idx) = existing_gs_idx {
                        // if this gamestate has already been seen before, it already has an index so we
                        // simply use that.
                        existing_gs_idx
                    } else {
                        // if this gamestate has *not* already been seen before, add it to all_gamestates
                        // and return the index we placed it at
                        let new_idx = all_gamestates.len();
                        all_gamestates.insert(new_idx, new_gs);
                        new_idx
                    };

                    // we always want to update the layer idx because we've either never seen this gamestate
                    // before, or we've seen it but at a different layer
                    all_gamestate_layers.insert(new_gs_idx, new_layer_idx);

                    // we always want to update how-we-got-here because we've either never seen this gamestate
                    // before and don't have a way to get to it yet, or we've seen it but with a
                    // longer chain of pours
                    tried_gamestates.insert(new_gs_idx, (gamestate_to_try_idx, pour));

                    // we always want to check what's below this gamestate because we've either never done that
                    // or we have, but didn't explore as far down.
                    gamestates_to_try.push(new_gs_idx);
                }
            }
        }
        println!(
            "checked {} gamestates and found no solution",
            tried_gamestates.len()
        );
        println!("visited counts: {:?}", visited_at_each_layer_count);
        None
    }

    pub fn get_base_gamestate(&self) -> &GameState<N> {
        self.base_gamestate
    }

    pub fn get_pours(&self) -> &VecDeque<Pour> {
        &self.pours
    }
}
