//! utilities for demystification of [PartialGameState]s into [KnownGameState](crate::gamestate::KnownGameState)s

use std::collections::{hash_map::Entry, HashMap};

use super::*;
use crate::{
    bottle::Bottle,
    colored_water::{ColoredWaterIter, ColoredWaterUnit, PartialColoredWaterUnit},
    gamestate::{GameState, PartialGameState}
};

/// Statistics regarding confidence in [try_demystify_next_step]'s result
#[derive(Default)]
pub struct DemystifyNextStepStats {
    /// The number of possible [Solution]s that were evaluated
    pub solutions_checked: usize,

    /// The smallest `pours_to_finish_estimate` found from any of the [Solution]s checked; i.e., the
    /// `pours_to_finish_estimate` of the returned [Solution]
    pub min_finished_estimate: usize,

    /// The number of possible [Solution]s that had the `min_finished_estimate`
    pub equal_solution_count: usize
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

    //find the solutions whose end state has the lowest pours to finish estimate
    let mut min_pours_estimate = usize::MAX;
    let mut min_scoring_solution = None;
    let mut equal_solution_count = 0;
    for possible_solution in possible_solutions {
        let mut working_gs = gamestate_to_solve.clone();
        for pour in possible_solution.get_pours() {
            working_gs = pour.try_apply(&working_gs).unwrap()
        }
        let base_score = working_gs.pours_to_finish_estimate();

        //increase the score by some amount proportional to how likely this state is to be a dead end
        //the penalty multiplier is 1.0 if the success chance is 1.0, and 2.0 if the success chance is 0.0
        let failure_chance_penalty_mult = (1.0 - rate_success_chance(&working_gs)) + 1.0;
        let score = ((base_score as f64) * failure_chance_penalty_mult) as usize;

        if score < min_pours_estimate {
            min_pours_estimate = score;
            min_scoring_solution = Some(possible_solution);
            equal_solution_count = 1;
        } else if score == min_pours_estimate {
            equal_solution_count += 1;
        }
    }

    min_scoring_solution.map(|s| {
        (
            s,
            DemystifyNextStepStats {
                solutions_checked,
                min_finished_estimate: min_pours_estimate,
                equal_solution_count
            }
        )
    })
}

/// Rate how likely it is for some partial gamestate to be possible to progress from by replacing the top unknown unit with each color,
/// trying to find a solution, and counting how many possibilities resulted in a solvable state.
///
/// A return value of 1.0 indicates a 100% chance of progression from the given state, and a return value of 0.0 returns a 0% chance of progression
/// (i.e. a 100% chance for a dead-end)
/// Returns 1.0 for gamestates with no top unit that is unknown.
///
/// Currently, does not consider how likely it is for the unknown unit to be any particular color; will skip colors that aren't
/// possible (because too many of that color are already known) but that's it. This aspect may be improved in the future.
///
/// If, somehow, there are multiple bottles whose topmost units are unknown, only tests with the first occurring unit
fn rate_success_chance<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gamestate_to_rate: &PartialGameState<MAX_BCOUNT, B_MAX_CAP>
) -> f64 {
    //first, get color counts for known units and max capacity of the largest bottle;
    //these will be used to ensure we don't add a unit of a color that would lead to too many units of that color
    let mut color_counts: HashMap<ColoredWaterUnit, usize> = HashMap::new();
    let mut largest_cap_seen = 0;
    for bottle in gamestate_to_rate.bottles.iter() {
        if bottle.capacity() > largest_cap_seen {
            largest_cap_seen = bottle.capacity();
        }
        if let Some(top_idx) = bottle.get_top_content_idx() {
            for idx in 0..=top_idx {
                if let Some(color) = bottle.sample_known_color_at(idx) {
                    match color_counts.entry(color) {
                        Entry::Occupied(mut e) => {
                            *e.get_mut() = e.get() + 1;
                        }
                        Entry::Vacant(e) => {
                            e.insert(1);
                        }
                    }
                }
            }
        }
    }

    //next, find the bottle with an unknown unit on top
    for (bottle_idx, bottle) in gamestate_to_rate.bottles.iter().enumerate() {
        if bottle.get_top_color() == Some(PartialColoredWaterUnit::UnknownColor) {
            //once we have our bottle, start trying colors
            let top_idx = bottle.get_top_content_idx().unwrap();

            let mut trial_count = 0_usize;
            let mut success_count = 0_usize;
            for color in ColoredWaterIter(None) {
                //determine if adding one unit of this color would lead to too many units of that color
                let this_color_current_count = *color_counts.get(&color).unwrap_or(&0);
                if this_color_current_count + 1 > largest_cap_seen {
                    //if it would, skip checking this color
                    continue;
                }

                let mut sim_state = gamestate_to_rate.clone();
                sim_state.bottles[bottle_idx]
                    .try_set_color(top_idx, Some(PartialColoredWaterUnit::Color(color)))
                    .unwrap();
                trial_count += 1;

                if Solution::try_new(&sim_state, 0).is_some() {
                    success_count += 1;
                }
            }

            let success_chance = (success_count as f64) / (trial_count as f64);
            return success_chance;
        }
    }
    //we reach here if there was no bottle with an unknown unit on top, return 1.0 by default
    1.0
}
