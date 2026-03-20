//! Utilities for automatically demystifying and solving a [PseudoPartialGameState] for automated testing

use std::time::{Duration, Instant};

use crate::{
    bottle::{Bottle, BottleSampleResult},
    colored_water::PartialColoredWaterUnit,
    gamestate::{GameState, KnownGameState, PartialGameState, PseudoPartialGameState},
    solution::{try_demystify_next_step, Solution}
};

/// The result of [auto_demystify]
#[derive(Debug)]
pub struct AutoDemystificationResult<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    /// The [DemystificationResult::current_state](crate::ui::DemystificationResult::current_state)
    /// after running the auto-demystify process.
    ///
    /// Note that the [DemystificationResult::initial_state](crate::ui::DemystificationResult::initial_state)
    /// isn't provided, since it will be identical
    /// to calling [PseudoPartialGameState::known] on the originally provided [PseudoPartialGameState]
    pub current_state: KnownGameState<MAX_BCOUNT, B_MAX_CAP>,

    /// Whether the `current_state` can be solved
    pub current_state_solvable: bool,

    /// The number of resets that were needed in order to complete demystification
    pub reset_count: usize,

    /// The number of times a demystification next-step was generated
    pub step_count: usize,

    /// The total number of pours used as part of demystification (does not include pours to an actual solution)
    pub total_pour_count: usize,

    /// The total amount of time spent generating demystification next-steps
    pub total_demystification_time: Duration,

    /// The largest amount of time spent generating any single demystification next-step
    pub max_demystification_time: Duration
}

/// Automatically run the demystification process on a given [PseudoPartialGameState] and
/// return statistics on how the demystification process went.
///
/// If `print_progress` is `true`, prints status messages as the demystification process runs. If `false`,
/// no output will be printed.
///
/// Useful mostly for evaluating demystification performance (both in terms of speed and number of resets required).
pub fn auto_demystify<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gs: PseudoPartialGameState<MAX_BCOUNT, B_MAX_CAP>,
    print_progress: bool
) -> AutoDemystificationResult<MAX_BCOUNT, B_MAX_CAP> {
    let (as_known, as_partial) = gs.take();

    //this will be the gamestate the user actually interacts with
    let mut working_gs = as_partial.clone();

    //this will be the gamestate that tracks what to reset to if we need to reset. we need it to be mutable
    //so we can update the unknown colors as they're revealed
    let mut initial_gs = as_partial;

    let mut reset_count = 0;
    let mut step_count = 0;
    let mut total_pour_count = 0;
    let mut total_demystification_time = Duration::default();
    let mut max_demystification_time = Duration::default();

    loop {
        //first thing we do: if the working state can be converted into a known state, do the conversion and return.
        if let Ok(working_state_as_known) = KnownGameState::try_from(working_gs.clone()) {
            //ensure our demystified initial_state matches our actual as_known state
            let calculated_as_known = KnownGameState::try_from(initial_gs)
            .expect("working state converted from partial to known, but initial state couldn't convert!");
            assert_eq!(
                calculated_as_known, as_known,
                "calculated state as known did not match actual as known"
            );

            let current_state_solvable = Solution::try_new(&working_state_as_known, 0).is_some();

            return AutoDemystificationResult {
                current_state: working_state_as_known,
                current_state_solvable,
                reset_count,
                step_count,
                total_pour_count,
                total_demystification_time,
                max_demystification_time
            };
        }

        //try to find a solution to this PartialGameState - this will be a partial state with at least one unknown color on top
        let demystification_start = Instant::now();
        let demystify_next_step = try_demystify_next_step(&working_gs);
        let demystification_duration = demystification_start.elapsed();
        total_demystification_time += demystification_duration;
        if demystification_duration > max_demystification_time {
            max_demystification_time = demystification_duration
        }
        step_count += 1;
        if print_progress {
            println!(
                "Demystification step {} took {:?}",
                step_count, demystification_duration
            );
        }
        if let Some((found_solution, stats)) = demystify_next_step {
            let pours = found_solution.take_pours();
            total_pour_count += pours.len();
            if print_progress {
                println!(
                    "Demystification step {} applied {} pour(s); {} solutions analyzed, found min score of {} (dead-end chance of {}), {} solution(s) with this score",
                    step_count,
                    pours.len(),
                    stats.solutions_checked,
                    stats.min_score,
                    stats.dead_end_chance,
                    stats.equal_scoring_solution_count
                )
            }

            // this var will track the final bottle poured from so we can limit
            // the unknown-to-known setup menu to that bottle, as it's the only one that
            // should now have an unknown unit on top.
            let mut last_source_idx = None;

            for pour in pours {
                last_source_idx = Some(pour.source_bottle_index);
                working_gs = pour
                    .try_apply(&working_gs)
                    .expect("invalid pour from solution");
            }
            // if last_source_idx was never set, (probably because the gamestate was solved
            // from the get-go and the solution was a no-op), we'll try to use the first bottle
            // with an unknown unit on top as a fallback
            if last_source_idx.is_none() {
                for (idx, bottle) in working_gs.bottles.iter().enumerate() {
                    if bottle.get_top_color() == Some(PartialColoredWaterUnit::UnknownColor) {
                        last_source_idx = Some(idx);
                        break;
                    }
                }
            }

            reveal_top_unknowns(&mut working_gs, &as_known);

            // identify unknown colors in initial_gs's bottle at last_source_idx
            // whose equivalents in working_gs are known
            if let Some(last_source_idx) = last_source_idx {
                let initial_bottle = initial_gs
                    .get_mut_bottles()
                    .get_mut(last_source_idx)
                    .expect("bottle in initial_gs at last_source_idx doesn't exist");
                let working_bottle = working_gs
                    .get_bottles()
                    .get(last_source_idx)
                    .expect("bottle in working_gs at last_source_idx doesn't exist");

                for color_idx in (0..initial_bottle.capacity()).rev() {
                    let initial_bottle_sample_result = initial_bottle.sample_at(color_idx);

                    if let BottleSampleResult::KnownColor(color) =
                        working_bottle.sample_at(color_idx)
                    {
                        if initial_bottle_sample_result == BottleSampleResult::UnknownColor {
                            initial_bottle
                                .try_set_color(
                                    color_idx,
                                    Some(PartialColoredWaterUnit::Color(color))
                                )
                                .expect(
                                    "Failed to update initial bottle with working bottle's content"
                                );
                        }
                    }
                }
            }
        } else {
            // no solution to the partial state could be found - we can't get anywhere useful from here
            // therefore, we must reset to our initial state and try again
            working_gs = initial_gs.clone();
            reset_count += 1;
            if print_progress {
                println!("Required reset after demystification step {}", step_count);
            }
        }
    }
}

/// Look at unknown units at the top of bottles inside `working_gs`,
/// and replace said units with known units from `as_known`, as though a user
/// had manually entered them
///
/// `as_known` must correspond to the original [PartialGameState] that `working_gs` came from.
/// This property isn't checked, and bad things will happen if it's not true, so be careful when using this function.
fn reveal_top_unknowns<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    working_gs: &mut PartialGameState<MAX_BCOUNT, B_MAX_CAP>,
    as_known: &KnownGameState<MAX_BCOUNT, B_MAX_CAP>
) {
    for (working_bottle, known_bottle) in working_gs.bottles.iter_mut().zip(as_known.bottles.iter())
    {
        if let Some(top_content_idx) = working_bottle.get_top_content_idx() {
            if working_bottle.sample_content_at(top_content_idx)
                == Some(PartialColoredWaterUnit::UnknownColor)
            {
                //for each c_idx where the color in the known bottle matches the color at the top unknown unit,
                //set the unknown color to the known color. this emulates revealing multiple units at a time
                //when there are multiple of the same color in a row
                let color = known_bottle.sample_known_color_at(top_content_idx).unwrap();
                let mut c_idx = top_content_idx;
                while known_bottle.sample_known_color_at(c_idx) == Some(color) {
                    //set the color at c_idx to the known color
                    working_bottle
                        .try_set_color(c_idx, Some(PartialColoredWaterUnit::Color(color)))
                        .expect("setting color failed while revealing top unknowns");

                    //decrement c_idx, or break out of while loop if c_idx is currently 0
                    if let Some(new_c_idx) = c_idx.checked_sub(1) {
                        c_idx = new_c_idx;
                    } else {
                        break;
                    }
                }
            }
        }
    }
}
