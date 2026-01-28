//! utilities for demystification of [PartialGameState]s into [KnownGameState](crate::gamestate::KnownGameState)s

use super::*;
use crate::gamestate::PartialGameState;
use crossbeam_channel;
use std::{
    collections::{hash_map::Entry, HashMap},
    num::NonZeroUsize,
    thread::{self, available_parallelism}
};

const DEFAULT_THREADCOUNT: NonZeroUsize = NonZeroUsize::new(4).unwrap();

/// Statistics regarding confidence in [try_demystify_next_step]'s result
#[derive(Default)]
pub struct DemystifyNextStepStats {
    /// The number of possible [KnownGameState](crate::gamestate::KnownGameState)s that
    /// were checked for a solution.
    pub possible_states_checked: usize,

    /// The number of possible [KnownGameState](crate::gamestate::KnownGameState)s that
    /// were solved in order to find the next step. This will always be less than or equal to `possible_states_checked`.
    pub solutions_found: usize,

    /// The number of possible [KnownGameState](crate::gamestate::KnownGameState)s whose
    /// solutions start with the provided `next_step`. This will always be less than or equal to `solutions_found`.
    /// The higher this number is, the more confidence there can be in the `next_step` being of good quality.
    pub solutions_sharing_prefix: usize
}

/// Try to find a [Solution] for the given [PartialGameState] that leads to revealing a new unknown color unit
/// while using a prediction technique to try and prevent dead-ends. Similar to [Solution::try_new], will return [None]
/// if no [Solution] can be found.
///
/// This returns the found [Solution] along with an instance of [DemystifyNextStepStats] so the level of confidence
/// in the answer can be communicated
pub fn try_demystify_next_step<'a, const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gamestate_to_solve: &'a PartialGameState<MAX_BCOUNT, B_MAX_CAP>
) -> Option<(
    Solution<'a, PartialGameState<MAX_BCOUNT, B_MAX_CAP>>,
    DemystifyNextStepStats
)> {
    let mut pours = Vec::new();
    let mut working_gs = gamestate_to_solve.clone();
    let mut possible_states_checked = 0;
    let mut solutions_found = 0;
    let mut solutions_sharing_prefix = 0;
    loop {
        //if we have a solution, return it
        if let Ok(solution) = Solution::try_from_parts(gamestate_to_solve, pours.iter().cloned()) {
            return Some((
                solution,
                DemystifyNextStepStats {
                    possible_states_checked,
                    solutions_found,
                    solutions_sharing_prefix
                }
            ));
        }

        //try finding the immediate next pour
        if let Some((next_pour, stats)) = get_next_pour(&working_gs) {
            //apply this pour to our working_gs and add it to our list of pours
            working_gs = next_pour.try_apply(&working_gs).unwrap();
            pours.push(next_pour);
            //add our stats
            possible_states_checked += stats.possible_states_checked;
            solutions_found += stats.solutions_found;
            solutions_sharing_prefix += stats.solutions_sharing_prefix;
        } else {
            //no immediate next pour could be found, and since we reached this point, we know that our current pours
            //aren't a solution. therefore, we must bail out and try the fallback strategy
            break;
        }
    }
    //if we reach this point, no next pour could be found. as a fallback, try just generating any path to the next unknown color.
    Solution::try_new(gamestate_to_solve, 0).map(|s| (s, DemystifyNextStepStats::default()))
}

/// Find the immediate next pour for the given [PartialGameState] that will be used as part of a [Solution].
/// Returns [None] if no next pour can be found.
fn get_next_pour<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gamestate_to_solve: &PartialGameState<MAX_BCOUNT, B_MAX_CAP>
) -> Option<(Pour, DemystifyNextStepStats)> {
    // this is the number of possible gamestates we'll sample and try to solve
    // in theory, the bigger this number, the more accurate our predictions will
    // be and the better we'll be at avoiding dead ends.
    // however, it also makes the function take a considerable amount more time
    // to run, and increasing yields diminishing returns to some extent
    const SAMPLE_SIZE: usize = 10;

    // try to get a sample of possible gamestates
    // if this fails, our prediction technique won't work
    if let Some(possible_gamestates) = gamestate_to_solve.collapse(SAMPLE_SIZE) {
        // solve all those sample gamestates in multiple threads

        //thread count is either the number of available threads (or a default of 4 if available_parallelism fails),
        //or the number of gamestates we have to solve, whichever is smaller
        let thread_count: usize = possible_gamestates.len().min(
            available_parallelism()
                .unwrap_or(DEFAULT_THREADCOUNT)
                .into()
        );
        //create some channels to send unsolved gamestates from our main thread to workers, and send solutions
        //from our workers to our main thread
        let (send_work, recv_work) = crossbeam_channel::unbounded();
        let (send_solution, recv_solution) = crossbeam_channel::unbounded();

        let mut solutions = Vec::new();
        thread::scope(|s| {
            //spawn worker threads
            for _ in 0..thread_count {
                //each worker gets its own work receiver and result sender
                let thread_recv = recv_work.clone();
                let thread_send = send_solution.clone();
                s.spawn(move || {
                    //while a work sender exists, wait to get a new gamestate and then solve it
                    while let Ok(possible_gamestate) = thread_recv.recv() {
                        if let Some(solution) = Solution::try_new(possible_gamestate, 0) {
                            //if we found a solution, send it back through the solution sender
                            thread_send
                                .send(solution)
                                .expect("main thread hung up before worker completed");
                        }
                    }
                });
            }

            //send all the possible gamestates to the workers
            for possible_gamestate in possible_gamestates.iter() {
                send_work
                    .send(possible_gamestate)
                    .expect("worker threads hung up before all states processed");
            }

            //drop the work sender so that threads know there won't be any more work to do.
            drop(send_work);
            //drop our copy of the result sender so that only the workers' copies will remain
            drop(send_solution);
        });

        //receive all the solutions from the workers
        while let Ok(solution) = recv_solution.recv() {
            solutions.push(solution);
        }

        let total_solutions_found = solutions.len();
        if total_solutions_found == 0 {
            //if we found no solutions, that could be because it's impossible to progress,
            //because it's possible to progress to another unknown color, but not possible to win,
            //or because we got unlucky and picked only unwinnable possible states even though there are
            //winnable states. to cover the second two cases, if we found no solutions, we'll try to find and return
            //the beginning to any path to revealing an unknown color, even if it's not part of any solution.
            return Solution::try_new(gamestate_to_solve, 0)
                .and_then(|s| s.take_pours().into_iter().next())
                .map(|p| {
                    (
                        p,
                        DemystifyNextStepStats {
                            possible_states_checked: possible_gamestates.len(),
                            ..Default::default()
                        }
                    )
                });
        }

        // we organize all solutions by their first pour, pick the most common pour, then return it
        let solutions_by_pour = group_solutions_by_pour(solutions.into_iter(), 0);
        let mut most_common_move: Option<(Pour, Vec<Solution<'_, _>>)> = None;
        for (pour, solutions) in solutions_by_pour.into_iter() {
            if let Some((_, other_solutions)) = most_common_move.as_ref() {
                if solutions.len() > other_solutions.len() {
                    most_common_move = Some((pour, solutions))
                }
            } else {
                most_common_move = Some((pour, solutions))
            }
        }

        if let Some((pour, solutions)) = most_common_move.take() {
            return Some((
                pour,
                DemystifyNextStepStats {
                    possible_states_checked: possible_gamestates.len(),
                    solutions_found: total_solutions_found,
                    solutions_sharing_prefix: solutions.len()
                }
            ));
        } else {
            //if we reach here, there must not have been any pours in any of the solutions;
            //there's nothing else we can do here, so we return None.
            return None;
        }
    }
    //if we reach here, we couldn't generate possible gamestates
    //this means our prediction technique can't work, so we fall back to using the simple method of just
    //finding any path to the next color with no regard for dead ends and using the first pour from that path
    Solution::try_new(gamestate_to_solve, 0)
        .and_then(|s| s.take_pours().into_iter().next())
        .map(|p| (p, DemystifyNextStepStats::default()))
}

/// Given some iterator over Solutions and a Pour index, builds a HashMap where keys
/// are pours from the given solutions, and values are vecs of solutions with the relevant pour.
///
/// For example, `solutions_by_pour(some_iter, 0)` returns a HashMap of all solutions in `some_iter`
/// grouped by their first pour (the pour at index 0). `solutions_by_pour(some_iter, 10)` returns
/// a HashMap of all solutions in `some_iter` grouped by their 11th pour (the pour at index 10).
///
/// If any Solution has no pour at the given `pour_idx` it is simply not included in the returned HashMap.
fn group_solutions_by_pour<'a, T, GamestateT: SolvableGameState>(
    solutions: T,
    pour_idx: usize
) -> HashMap<Pour, Vec<Solution<'a, GamestateT>>>
where
    T: Iterator<Item = Solution<'a, GamestateT>>
{
    let mut mapping: HashMap<Pour, Vec<Solution<GamestateT>>> = HashMap::new();
    for solution in solutions {
        if let Some(next_pour) = solution.get_pours().get(pour_idx) {
            match mapping.entry(next_pour.clone()) {
                Entry::Occupied(mut e) => {
                    e.get_mut().push(solution);
                }
                Entry::Vacant(e) => {
                    e.insert(vec![solution]);
                }
            }
        }
    }
    mapping
}
