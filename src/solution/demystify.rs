//! utilities for demystification of [PartialGameState]s into [KnownGameState](crate::gamestate::KnownGameState)s

use super::*;
use crate::gamestate::PartialGameState;
use std::collections::{hash_map::Entry, HashMap, VecDeque};

/// Try to find a [Solution] for the given [PartialGameState] that leads to revealing a new unknown color unit
/// while using a prediction technique to try and prevent dead-ends. Similar to [Solution::try_new], will return [None]
/// if no [Solution] can be found.
pub fn try_demystify_next_step<'a, const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gamestate_to_solve: &'a PartialGameState<MAX_BCOUNT, B_MAX_CAP>
) -> Option<Solution<'a, PartialGameState<MAX_BCOUNT, B_MAX_CAP>>> {
    // this is the number of possible gamestates we'll sample and try to solve
    // in theory, the bigger this number, the more accurate our predictions will
    // be and the better we'll be at avoiding dead ends.
    // however, it also makes the function take a considerable amount more time
    // to run, and increasing may yield diminishing returns to some extent.
    const SAMPLE_SIZE: usize = 100;

    // try to get a sample of possible gamestates
    // if this fails, our prediction technique won't work
    if let Some(possible_gamestates) = gamestate_to_solve.collapse(SAMPLE_SIZE) {
        // solve all those sample gamestates
        // TODO: this part is very slow. find a way to speed it up, or switch this algorithm out for one that doesn't
        // need full-depth solutions for this many states
        let mut solutions = Vec::new();
        for possible_gs in possible_gamestates.iter() {
            if let Some(this_solution) = Solution::try_new(possible_gs, 0) {
                solutions.push(this_solution);
            }
        }

        // we want to find the most common first pours, so we'll organize all solutions by their first pour,
        // pick the most common pour to add to our pours vec, remove all solutions that didn't have that first pour,
        // and then repeat with the second pour. we'll keep repeating with the third, fourth, etc. until we have a list of
        // pours that leads to our original gamestate being solved (i.e. having an unknown color on top somewhere)
        let mut solutions_by_pour = group_solutions_by_pour(solutions.into_iter(), 0);
        let mut pours = VecDeque::new();
        loop {
            //first, see if applying these pours to our initial gamestate results in a solution we can return
            //if it does, return now.
            if let Ok(solution) =
                Solution::try_from_parts(gamestate_to_solve, pours.iter().cloned())
            {
                return Some(solution);
            }

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
                pours.push_back(pour.clone());
                //we now need to reset solutions_by_pour to contain solutions by their next pour
                solutions_by_pour = group_solutions_by_pour(solutions.into_iter(), pours.len());
            } else {
                //no moves, no solution is possible
                return None;
            }
        }
    }
    //if we reach here, we couldn't generate possible gamestates
    //this means our prediction technique can't work, so we fall back to using the simple method of just
    //finding any path to the next color with no regard for dead ends
    Solution::try_new(gamestate_to_solve, 0)
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
