//! Implementation of a [Solution]

use std::{
    cmp::min,
    collections::{HashMap, VecDeque},
    io::{self, Write},
    num::NonZeroUsize,
    sync::{Arc, RwLock},
    thread::{available_parallelism, spawn},
    time::Instant
};

use bimap::BiHashMap;

use crate::gamestate::{GameState, Pour};

type ThreadOutput = Arc<RwLock<Option<VecDeque<Pour>>>>;

/// A series of [Pour]s that, when applied to some [GameState]
/// in sequence, results in a GameState that is finished.
///
/// Although represented as Pours, each Pour is guaranteed
/// to be valid for the result of the previous Pour (except the first Pour, which
/// is instead guaranteed to be valid for the provided `base_gamestate`)
pub struct Solution<'a, const BCOUNT: usize, const BSIZE: usize> {
    base_gamestate: &'a GameState<BCOUNT, BSIZE>,
    pours: VecDeque<Pour>
}

impl<'a, const BCOUNT: usize, const BSIZE: usize> Solution<'a, BCOUNT, BSIZE> {
    /// Try to find and return the shortest possible Solution to the given GameState. If no solution can be found, return `None`.
    ///
    /// Will search for solutions with as many as `max_depth` pours. If no solution is found within that many pours, returns `None`.
    /// Setting `max_depth` to 0 disables the limit; will search for possible solutions that are arbitrarily long until a valid solution is found,
    /// all possible solutions have been checked without a valid solution, or the program panics due to the computer being out of memory.
    ///
    /// Will always return a Solution with the fewest possible pours and theoretically
    /// uses less memory than [Solution::try_new_threaded], but is usually slower.
    pub fn try_new(base_gamestate: &'a GameState<BCOUNT, BSIZE>, max_depth: u8) -> Option<Self> {
        Self::find_solving_pours_breadth_first(base_gamestate, max_depth).map(|pours| Self {
            base_gamestate,
            pours
        })
    }

    /// Try to find and return a Solution to the given GameState. If no solution can be found, return `None`.
    ///
    /// Will search for solutions with as many as `max_depth` pours. If no solution is found within that many pours, returns `None`.
    /// Setting `max_depth` to 0 disables the limit; will search for possible solutions that are arbitrarily long until a valid solution is found,
    /// all possible solutions have been checked without a valid solution, or the program panics due to the computer being out of memory.
    ///
    /// Generally faster than [Solution::try_new], and will usually return a Solution that either is, or is close to, the shortest possible solution.
    /// However, that function typically uses less memory, and is guaranteed to return the shortest possible solution.
    pub fn try_new_threaded(
        base_gamestate: &'a GameState<BCOUNT, BSIZE>,
        max_depth: u8
    ) -> Option<Self> {
        Self::find_solving_pours_threaded(base_gamestate, max_depth).map(|pours| Self {
            base_gamestate,
            pours
        })
    }

    fn find_solving_pours_threaded(
        gamestate_to_solve: &GameState<BCOUNT, BSIZE>,
        max_depth: u8
    ) -> Option<VecDeque<Pour>> {
        const DEFAULT_MAX_THREAD_COUNT: usize = 4;
        let max_thread_count = available_parallelism().unwrap_or_else(|_| {
            eprintln!("Warning: CPU count could not be queried with std::thread::available_parallelism; using default of {}", DEFAULT_MAX_THREAD_COUNT);
            NonZeroUsize::new(DEFAULT_MAX_THREAD_COUNT).unwrap()
        }).get();

        let initial_gamestates: Vec<(GameState<BCOUNT, BSIZE>, Pour)> = gamestate_to_solve
            .iter_pours()
            .map(|vp| (vp.apply(), vp.into()))
            .collect();

        // when max_depth is 1, we can't spawn any threads as they'd operate starting at depth 2;
        // instead we just check if our initial_gamestates contains any winners and return what we find, if anything
        if max_depth == 1 {
            for (gs, pour) in initial_gamestates {
                if gs.is_finished() {
                    let mut out = VecDeque::new();
                    out.push_front(pour);
                    return Some(out);
                }
            }
            return None;
        }

        let mut handles = Vec::new();

        //this is where the workers will write their results to, if they have any
        let thread_output = Arc::new(RwLock::new(None));

        let thread_count = min(initial_gamestates.len(), max_thread_count);

        let worker_max_depth = if max_depth == 0 { 0 } else { max_depth - 1 };
        //create workers
        let chunked_gamestates = initial_gamestates[..]
            .chunks((initial_gamestates.len() + (thread_count - 1)) / thread_count);
        let mut chunk_start_idx = 0;
        for gamestate_chunk in chunked_gamestates {
            let gamestate_chunk: Vec<GameState<BCOUNT, BSIZE>> =
                gamestate_chunk.iter().map(|(gs, _)| gs.clone()).collect();

            let chunk_len = gamestate_chunk.len();
            let thread_output = thread_output.clone();

            handles.push(spawn(move || {
                Self::find_solving_pours_threaded_worker(
                    gamestate_chunk,
                    worker_max_depth,
                    chunk_start_idx,
                    thread_output
                )
            }));
            chunk_start_idx += chunk_len;
        }

        //wait for each thread to complete, in order.
        //as soon as any thread finds a solution, the rest will exit after their next loop iteration
        //so we won't wait by too much more than we need to.
        //worst-case, there is no solution, and we wait for every thread to fully
        //verify that its gamestates were unsolvable
        for handle in handles {
            // if this thread found the solution, it will return the idx of the gamestate/pour that it was checking
            // when it found the solution

            if let Some(winning_pour_idx) = handle.join().unwrap() {
                let mut solution = thread_output.read().unwrap().clone().unwrap();
                let winning_pour = initial_gamestates.get(winning_pour_idx).unwrap().1.clone();
                solution.push_front(winning_pour);
                return Some(solution);
            }
        }

        None
    }

    /// Function that will be run by individual threads under find_solving_pours_threaded
    fn find_solving_pours_threaded_worker(
        gamestates_to_solve: Vec<GameState<BCOUNT, BSIZE>>,
        max_depth: u8,
        base_idx: usize,
        output: ThreadOutput
    ) -> Option<usize> {
        for (gamestate_to_solve_idx, gamestate_to_solve) in
            gamestates_to_solve.into_iter().enumerate()
        {
            // only generate GameStates once and reference them from here by index.
            // GameStateIdx is an alias to some type we can use to uniquely index into this HashMap;
            // We use an alias to make it more clear what we're doing
            type GameStateIdx = usize;
            let mut all_gamestates: BiHashMap<GameStateIdx, GameState<BCOUNT, BSIZE>> =
                BiHashMap::new();
            all_gamestates.insert(0, gamestate_to_solve.clone()); // index is 0

            //map from gamestate to (source_gamestate, pour_for_source_gamestate)
            //this allows us to track gamestates
            let mut tried_gamestates: HashMap<GameStateIdx, (GameStateIdx, Pour)> = HashMap::new();
            let mut gamestates_to_try: VecDeque<(u8, GameStateIdx)> = VecDeque::new();
            gamestates_to_try.push_back((0, 0)); // add our starting gamestate

            while let Some((layer_idx, gamestate_to_try_idx)) = gamestates_to_try.pop_front() {
                //end now if some thread has found a solution
                if output.read().unwrap().is_some() {
                    return None;
                }
                let gamestate_to_try = all_gamestates.get_by_left(&gamestate_to_try_idx).unwrap();
                if gamestate_to_try.is_finished() {
                    let mut returnable = VecDeque::new();
                    let mut gs_idx = gamestate_to_try_idx;
                    loop {
                        if let Some((source_gs_idx, pour)) = tried_gamestates.remove(&gs_idx) {
                            returnable.push_front(pour);
                            gs_idx = source_gs_idx;
                        } else {
                            //write out our solution and exit
                            *output.write().unwrap() = Some(returnable);
                            return Some(base_idx + gamestate_to_solve_idx);
                        }
                    }
                }

                // only explore deeper than this if the max_depth limit is
                // disabled or we aren't yet at the max_depth
                if layer_idx < max_depth || max_depth == 0 {
                    // the following could theoretically be done in one loop, but borrow rules prevent this.
                    // instead of just cloning, we first create a vector of all new entries that will go into all_gamestates
                    let mut new_pours: Vec<(GameState<BCOUNT, BSIZE>, Pour)> = Vec::new();
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
                        // we use saturating add for the new layer_idx to ensure that if we reach layer 255,
                        // items on layer 256 aren't said to be on layer 0; if the layer idx is going to
                        // be incorrect, we'd prefer it to at least never go down
                        gamestates_to_try.push_back((layer_idx.saturating_add(1), new_gs_idx));
                    }
                }
            }
        }
        None
    }

    /// Given a GameState, generate possible pours and identify a list of pours that,
    /// when applied in order, leads to a solution.
    ///
    /// Returns None if there are no solutions.
    fn find_solving_pours_breadth_first(
        gamestate_to_solve: &GameState<BCOUNT, BSIZE>,
        max_depth: u8
    ) -> Option<VecDeque<Pour>> {
        const PRINT_METRICS: bool = true;

        // only generate GameStates once and reference them from here by index.
        // GameStateIdx is an alias to some type we can use to uniquely index into this HashMap;
        // We use an alias to make it more clear what we're doing
        type GameStateIdx = usize;
        let mut all_gamestates: BiHashMap<GameStateIdx, GameState<BCOUNT, BSIZE>> =
            BiHashMap::new();
        all_gamestates.insert(0, gamestate_to_solve.clone()); // index is 0

        //map from gamestate to (source_gamestate, pour_for_source_gamestate)
        //this allows us to track gamestates
        let mut tried_gamestates: HashMap<GameStateIdx, (GameStateIdx, Pour)> = HashMap::new();
        let mut gamestates_to_try: VecDeque<(u8, GameStateIdx)> = VecDeque::new();
        gamestates_to_try.push_back((0, 0)); // add our starting gamestate

        let overall_start_time = if PRINT_METRICS {
            Some(Instant::now())
        } else {
            None
        };
        let mut layer_start_time = overall_start_time;
        let mut last_seen_layer_idx: Option<u8> = if PRINT_METRICS { Some(0) } else { None };
        let mut states_within_layer: Option<usize> = if PRINT_METRICS { Some(0) } else { None };

        while let Some((layer_idx, gamestate_to_try_idx)) = gamestates_to_try.pop_front() {
            if PRINT_METRICS {
                if layer_idx != last_seen_layer_idx.unwrap() {
                    if last_seen_layer_idx.unwrap() != 0 {
                        let layer_end_time = Instant::now();
                        println!(
                            "Done; ({} members processed in {:?}) (overall time: {:?})",
                            states_within_layer.unwrap() + 1,
                            layer_end_time.duration_since(layer_start_time.unwrap()),
                            layer_end_time.duration_since(overall_start_time.unwrap())
                        );
                        layer_start_time = Some(layer_end_time)
                    }
                    print!("Starting layer with index {}... ", layer_idx);
                    io::stdout().flush().unwrap();
                    last_seen_layer_idx = Some(layer_idx);
                    states_within_layer = Some(0);
                } else {
                    states_within_layer = Some(states_within_layer.unwrap() + 1);
                }
            }
            let gamestate_to_try = all_gamestates.get_by_left(&gamestate_to_try_idx).unwrap();
            if gamestate_to_try.is_finished() {
                if PRINT_METRICS {
                    let layer_end_time = Instant::now();
                    println!(
                        "Found solution! ({} members processed in {:?}) ({} members in layer)",
                        states_within_layer.unwrap(),
                        layer_end_time.duration_since(layer_start_time.unwrap()),
                        states_within_layer.unwrap()
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
                        layer_end_time.duration_since(overall_start_time.unwrap())
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

            // only explore deeper than this if the max_depth limit is
            // disabled or we aren't yet at the max_depth
            if layer_idx < max_depth || max_depth == 0 {
                // the following could theoretically be done in one loop, but borrow rules prevent this.
                // instead of just cloning, we first create a vector of all new entries that will go into all_gamestates
                let mut new_pours: Vec<(GameState<BCOUNT, BSIZE>, Pour)> = Vec::new();
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
                    // we use saturating add for the new layer_idx to ensure that if we reach layer 255,
                    // items on layer 256 aren't said to be on layer 0; if the layer idx is going to
                    // be incorrect, we'd prefer it to at least never go down
                    gamestates_to_try.push_back((layer_idx.saturating_add(1), new_gs_idx));
                }
            }
        }

        // we iterated through all possible pours from this gamestate and found no pours that
        // lead to a solution; it's not possible to win from here
        if PRINT_METRICS {
            let end_time = Instant::now();
            println!(
                "Done; ({} members processed in {:?})",
                states_within_layer.unwrap(),
                end_time.duration_since(layer_start_time.unwrap()),
            );
            println!(
                "Overall, evaluated {} members in {:?}, but did not find a solution",
                tried_gamestates.len(),
                end_time.duration_since(overall_start_time.unwrap())
            );
        }
        None
    }

    pub fn get_base_gamestate(&self) -> &GameState<BCOUNT, BSIZE> {
        self.base_gamestate
    }

    pub fn get_pours(&self) -> &VecDeque<Pour> {
        &self.pours
    }
}
