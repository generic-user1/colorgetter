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
/// Does consider how likely each color is to appear, see [rate_color_probabilities].
///
/// If, somehow, there are multiple bottles whose topmost units are unknown, only tests with the first occurring unit.
fn rate_success_chance<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gamestate_to_rate: &PartialGameState<MAX_BCOUNT, B_MAX_CAP>
) -> f64 {
    //find the bottle with an unknown unit on top
    for (bottle_idx, bottle) in gamestate_to_rate.bottles.iter().enumerate() {
        if bottle.get_top_color() == Some(PartialColoredWaterUnit::UnknownColor) {
            //once we have our bottle, calculate probabilities of this unit being each color, then start trying colors
            let color_probabilities = rate_color_probabilities(gamestate_to_rate);
            let top_idx = bottle.get_top_content_idx().unwrap();

            //this will be the sum of the probabilities of any successful outcome
            let mut success_chance = 0.0;

            //sim_state will be the state we set a color in and try to find a solution for.
            //since we're only making one change, and that change is in the same spot each time, we only
            //actually need one sim_state for all tests and so we create it outside of the loop.
            //because we only need one sim_state, we could technically save this clone by taking
            //gamestate_to_rate instead of borrowing it, but weirdly, borrowing it and cloning appears to be slightly faster
            let mut sim_state = gamestate_to_rate.clone();
            for color in ColoredWaterIter(None) {
                //if this color has a 0 probability or isn't in the probabilities map, skip it entirely
                match color_probabilities.get(&color).copied() {
                    None => continue,
                    Some(0.0) => continue,
                    Some(probability) => {
                        sim_state.bottles[bottle_idx]
                            .try_set_color(top_idx, Some(PartialColoredWaterUnit::Color(color)))
                            .unwrap();
                        if Solution::try_new(&sim_state, 0).is_some() {
                            success_chance += probability;
                        }
                    }
                }
            }

            //our result should be a number between 0.0 and 1.0, but imprecision may mean it's slightly out of bounds, so we clamp before returning
            return success_chance.clamp(0.0, 1.0);
        }
    }
    //we reach here if there was no bottle with an unknown unit on top, return 1.0 by default
    1.0
}

/// Given a gamestate, determine how likely it is that any one unknown color unit will end up being
/// any of the posssible colors, based on how many appearances each color has.
///
/// Return value is a [HashMap] of [ColoredWaterUnit] keys to [f64] values where each value is a number from 0.0 to 1.0,
/// with 0.0 indicating 0% chance of an unknown unit being that color and 1.0 indicating a 100% chance of an unknown unit being that color.
///
/// Mathematically, all the values in the return value should add up to precisely 1.0, though floating-point imprecision
/// means they may add up to slightly more or less than this.
///
/// Makes some assumptions about the gamestate:
///
/// - The maximum number of units for any given color is the same as the capacity of the largest bottle.
///   If bottles are of varying capacity, this means the estimations returned by this function will be incorrect in some way.
///   This could lead to panicking - this issue may be resolved in the future
///
/// - If there are enough unknown units that all units of any particular color are unknown, this function will make a guess
///   as to which color/colors are entirely unknown. This guess will probably be wrong, but the probabilities should still make sense.
///
/// - There is at least one unknown unit (if there aren't any unknown units, division by zero could occur)
fn rate_color_probabilities<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gamestate_to_rate: &PartialGameState<MAX_BCOUNT, B_MAX_CAP>
) -> HashMap<ColoredWaterUnit, f64> {
    //first, get color counts for known units, max capacity of the largest bottle,
    //and total number of unknown units
    let mut known_color_counts: HashMap<ColoredWaterUnit, usize> = HashMap::new();
    let mut largest_cap_seen = 0;
    let mut unknown_unit_count = 0_usize;
    for bottle in gamestate_to_rate.bottles.iter() {
        if bottle.capacity() > largest_cap_seen {
            largest_cap_seen = bottle.capacity();
        }
        if let Some(top_idx) = bottle.get_top_content_idx() {
            for idx in 0..=top_idx {
                match bottle.sample_content_at(idx) {
                    Some(PartialColoredWaterUnit::Color(color)) => {
                        match known_color_counts.entry(color) {
                            Entry::Occupied(mut e) => {
                                *e.get_mut() = e.get() + 1;
                            }
                            Entry::Vacant(e) => {
                                e.insert(1);
                            }
                        }
                    }
                    Some(PartialColoredWaterUnit::UnknownColor) => {
                        unknown_unit_count += 1;
                    }
                    None => ()
                }
            }
        }
    }

    //now we will find the number of missing units for each color, assuming
    //that there should be `largest_cap_seen` units of each color
    let mut assumed_unknown_counts: HashMap<ColoredWaterUnit, usize> = HashMap::new();
    for (color, known_count) in known_color_counts.into_iter() {
        if known_count >= largest_cap_seen {
            assumed_unknown_counts.insert(color, 0);
        } else {
            assumed_unknown_counts.insert(color, largest_cap_seen - known_count);
        }
    }
    //it's possible there are colors entirely hidden - we check for this by comparing the sum of assumed_unknown_counts' values
    //to the total number of unknown units observed, and adding new colors until we meet the number of unknown units
    'outer: while assumed_unknown_counts.values().sum::<usize>() < unknown_unit_count {
        for possible_color in ColoredWaterIter(None) {
            if let Entry::Vacant(e) = assumed_unknown_counts.entry(possible_color) {
                e.insert(largest_cap_seen);
                continue 'outer;
            }
        }
        //if we reach this point, we ran out of possible colors before reaching our unknown unit count
        panic!("not enough colors to satisfy all unknown units")
    }

    //double check that our values line up
    assert_eq!(
        assumed_unknown_counts.values().sum::<usize>(),
        unknown_unit_count,
        "assumed unknown counts didn't sum to actual unknown count"
    );

    //now we know how many of each color there are, we can get probabilities for each color
    let unknown_unit_count = unknown_unit_count as f64;
    let mut probabilities = HashMap::new();
    for (color, count) in assumed_unknown_counts.into_iter() {
        probabilities.insert(color, (count as f64) / unknown_unit_count);
    }
    probabilities
}
