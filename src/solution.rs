//! Implementation of a [Solution]

use std::{
    collections::{HashMap, VecDeque},
    num::NonZeroUsize,
    sync::{Arc, RwLock},
    thread::{available_parallelism, spawn}
};

use bimap::BiHashMap;

use crate::gamestate::{GameState, Pour};

type ThreadOutput = Arc<RwLock<Option<VecDeque<Pour>>>>;

/// Represents the state of a solution finding algorithm
struct SolutionState<const BCOUNT: usize, const BSIZE: usize> {
    pub all_gamestates: BiHashMap<usize, GameState<BCOUNT, BSIZE>>,
    pub tried_gamestates: HashMap<usize, (usize, Pour)>,
    pub gamestates_to_try: VecDeque<(u8, usize)>
}

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
        let mut solution_state = Self::gen_initial_state(base_gamestate);
        let output = Arc::new(RwLock::new(None));

        Self::find_solving_pours_worker(
            Vec::from([&mut solution_state]),
            max_depth,
            0,
            output.clone(),
            true
        );
        Arc::into_inner(output)
            .unwrap()
            .into_inner()
            .unwrap()
            .map(|pours| Self {
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
        let solution_state = Self::gen_initial_state(base_gamestate);
        Self::find_solving_pours_threaded(solution_state, max_depth).map(|pours| Self {
            base_gamestate,
            pours
        })
    }

    fn find_solving_pours_threaded(
        mut state: SolutionState<BCOUNT, BSIZE>,
        max_depth: u8
    ) -> Option<VecDeque<Pour>> {
        const DEFAULT_INITIAL_DEPTH: u8 = 10;

        //initial depth, if max_depth is set, is half the max depth or 1 (whichever is greater)
        let initial_depth = if max_depth > 0 {
            (max_depth / 2).max(1)
        } else {
            // use this as the initial depth when max_depth is unspecified
            DEFAULT_INITIAL_DEPTH
        };

        const DEFAULT_MAX_THREAD_COUNT: usize = 4;
        let max_thread_count = available_parallelism().unwrap_or_else(|_| {
            eprintln!("Warning: CPU count could not be queried with std::thread::available_parallelism; using default of {}", DEFAULT_MAX_THREAD_COUNT);
            NonZeroUsize::new(DEFAULT_MAX_THREAD_COUNT).unwrap()
        }).get();

        // our initial search depth will be our default, or the passed-in max_depth (whichever is smaller)
        // note that a max_depth of 0 is considered "no limit", so that's the highest value instead of the lowest
        // (this is similar to ace-high in a card game)
        let (initial_depth, use_max_depth) = if initial_depth < max_depth || max_depth == 0 {
            (initial_depth, false)
        } else {
            (max_depth, true)
        };

        //this is where the workers will write their results to, if they have any
        let thread_output = Arc::new(RwLock::new(None));

        // explore the first initial_depth layers before doing anything else.
        let early_solution = if Self::find_solving_pours_worker(
            Vec::from([&mut state]),
            initial_depth,
            0,
            thread_output.clone(),
            use_max_depth
        )
        .is_some()
        {
            Arc::into_inner(thread_output.clone())
                .unwrap()
                .into_inner()
                .unwrap()
        } else {
            None
        };
        // if we found something early, return it now.
        // if we didn't, and our initial_depth is the passed in max_depth, return None now.
        if early_solution.is_some() {
            return early_solution;
        } else if use_max_depth {
            return None;
        }
        let initial_gamestates_to_try: Vec<usize> =
            state.gamestates_to_try.iter().map(|i| i.1).collect();

        let mut handles = Vec::new();

        let thread_count = initial_gamestates_to_try.len().min(max_thread_count);

        let worker_max_depth = if max_depth == 0 {
            0
        } else {
            max_depth.checked_sub(initial_depth + 1).unwrap()
        };

        //create workers
        let chunked_gamestates = initial_gamestates_to_try
            .chunks((state.gamestates_to_try.len() + (thread_count - 1)) / thread_count);
        let mut chunk_start_idx = 0;
        for gamestate_chunk in chunked_gamestates {
            let mut starting_state_chunk: Vec<SolutionState<BCOUNT, BSIZE>> = gamestate_chunk
                .iter()
                .map(|gs_idx| {
                    Self::gen_initial_state(state.all_gamestates.get_by_left(gs_idx).unwrap())
                })
                .collect();

            let chunk_len = gamestate_chunk.len();
            let thread_output = thread_output.clone();

            handles.push(spawn(move || {
                Self::find_solving_pours_worker(
                    starting_state_chunk.iter_mut().collect(),
                    worker_max_depth,
                    chunk_start_idx,
                    thread_output,
                    true
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

            if let Some(winning_init_gs_idx) = handle.join().unwrap() {
                let mut solution = thread_output.read().unwrap().clone().unwrap();
                let winning_gs_idx = *initial_gamestates_to_try.get(winning_init_gs_idx).unwrap();
                let mut gs_idx = winning_gs_idx;
                loop {
                    if let Some((source_gs_idx, pour)) = state.tried_gamestates.remove(&gs_idx) {
                        solution.push_front(pour);
                        gs_idx = source_gs_idx;
                    } else {
                        return Some(solution);
                    }
                }
            }
        }

        None
    }

    /// For each given SolutionState in order, generate possible pours and
    /// identify a list of pours that, when applied in order, leads to a solution.
    /// Return value is `base_idx` + the index within `states` of the starting state that led to the solution.
    ///
    /// `max_depth` specifies how many layers deep to search. When zero, there is effectively no limit to how deep to search.
    ///
    /// `base_idx` is only useful when this function is used as a thread worker. Workers are dispatched on 'chunks' of a larger
    /// set of SolutionStates; `base_idx` represents the index of the first state in this particular worker's chunk. When this
    /// function is used single-threaded, `base_idx` should always be 0.
    ///
    /// `output` is the shared area that this worker will write its solution to (if it finds one). It also serves as a way
    /// for workers to communicate with each other that a solution has been found; the function checks here to see if another
    /// thread has already written out a solution; if one has, the function terminates immediately.
    ///
    /// The `cutoff` setting only applies when `max_depth` is nonzero (i.e. there is a depth limit set).
    /// When `cutoff` is true, the function will stop generating gamestates that would go beyond `max_depth`.
    /// When `cutoff` is false, the function will generate gamestates that are one layer beyond `max_depth`, but won't evaluate
    /// them; instead returning after it has evaluated the final layer.
    fn find_solving_pours_worker(
        states: Vec<&mut SolutionState<BCOUNT, BSIZE>>,
        max_depth: u8,
        base_idx: usize,
        output: ThreadOutput,
        cutoff: bool
    ) -> Option<usize> {
        for (starting_state_idx, state) in states.into_iter().enumerate() {
            while let Some((layer_idx, gamestate_to_try_idx)) = state.gamestates_to_try.pop_front()
            {
                //end now if some thread has found a solution
                if output.read().unwrap().is_some() {
                    return None;
                }
                let gamestate_to_try = state
                    .all_gamestates
                    .get_by_left(&gamestate_to_try_idx)
                    .unwrap();
                if layer_idx > max_depth {
                    // put the state we just popped off back onto gamestates_to_try so that someone looking
                    // at our solutionstate can see what we were about to do
                    state
                        .gamestates_to_try
                        .push_front((layer_idx, gamestate_to_try_idx));
                    return None;
                }
                if gamestate_to_try.is_finished() {
                    let mut returnable = VecDeque::new();
                    let mut gs_idx = gamestate_to_try_idx;
                    loop {
                        if let Some((source_gs_idx, pour)) = state.tried_gamestates.remove(&gs_idx)
                        {
                            returnable.push_front(pour);
                            gs_idx = source_gs_idx;
                        } else {
                            //write out our solution and exit
                            *output.write().unwrap() = Some(returnable);
                            return Some(base_idx + starting_state_idx);
                        }
                    }
                }

                // only explore deeper than this if the max_depth limit is
                // disabled or we aren't yet at the max_depth
                if layer_idx < max_depth || max_depth == 0 || !cutoff {
                    // the following could theoretically be done in one loop, but borrow rules prevent this.
                    // instead of just cloning, we first create a vector of all new entries that will go into all_gamestates
                    let mut new_pours: Vec<(GameState<BCOUNT, BSIZE>, Pour)> = Vec::new();
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
        None
    }

    /// Generates the initial state for a solution algorithm
    fn gen_initial_state(
        gamestate_to_solve: &GameState<BCOUNT, BSIZE>
    ) -> SolutionState<BCOUNT, BSIZE> {
        // only generate GameStates once and reference them from here by index.
        // GameStateIdx is an alias to some type we can use to uniquely index into this HashMap;
        // We use an alias to make it more clear what we're doing
        type GameStateIdx = usize;
        let mut all_gamestates: BiHashMap<GameStateIdx, GameState<BCOUNT, BSIZE>> =
            BiHashMap::new();
        all_gamestates.insert(0, gamestate_to_solve.clone()); // index is 0

        //map from gamestate to (source_gamestate, pour_for_source_gamestate)
        //this allows us to track gamestates
        let tried_gamestates: HashMap<GameStateIdx, (GameStateIdx, Pour)> = HashMap::new();
        let mut gamestates_to_try: VecDeque<(u8, GameStateIdx)> = VecDeque::new();
        gamestates_to_try.push_back((0, 0)); // add our starting gamestate

        SolutionState {
            all_gamestates,
            tried_gamestates,
            gamestates_to_try
        }
    }

    pub fn get_base_gamestate(&self) -> &GameState<BCOUNT, BSIZE> {
        self.base_gamestate
    }

    pub fn get_pours(&self) -> &VecDeque<Pour> {
        &self.pours
    }
}
