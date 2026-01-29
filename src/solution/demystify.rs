//! utilities for demystification of [PartialGameState]s into [KnownGameState](crate::gamestate::KnownGameState)s

use super::*;
use crate::gamestate::{GameState, PartialGameState};

/// Statistics regarding confidence in [try_demystify_next_step]'s result
#[derive(Default)]
pub struct DemystifyNextStepStats {
    /// The number of possible [Solution]s that were evaluated
    pub solutions_checked: usize,

    /// The largest `finished_estimate` found from any of the [Solution]s checked; i.e., the
    /// `finished_estimate` of the returned [Solution]
    pub max_finished_estimate: f64
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
    const MAX_SOLUTIONS: usize = 1000;

    // generate up to `MAX_SOLUTIONS` solutions
    let possible_solutions = find_many_solutions(gamestate_to_solve, 0, MAX_SOLUTIONS);
    let solutions_checked = possible_solutions.len();

    //find the solutions whose end state has the highest finished_estimate
    let mut max_finished_estimate = 0.0;
    let mut max_scoring_solution = None;
    for possible_solution in possible_solutions {
        let mut working_gs = gamestate_to_solve.clone();
        for pour in possible_solution.get_pours() {
            working_gs = pour.try_apply(&working_gs).unwrap()
        }
        let score = working_gs.finished_estimate();
        if score > max_finished_estimate {
            max_finished_estimate = score;
            max_scoring_solution = Some(possible_solution);
        }
    }

    max_scoring_solution.map(|s| {
        (
            s,
            DemystifyNextStepStats {
                solutions_checked,
                max_finished_estimate
            }
        )
    })
}
