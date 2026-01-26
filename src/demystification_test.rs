use std::{path::PathBuf, time::Duration};

use super::AppError;

use colorgetter::{
    gamestate::{load_partial_gamestate_from_file, KnownGameState, PseudoPartialGameState},
    solution::auto_demystify
};

/// Run the demystification test
pub fn demystification_test(
    gamestate_file_path: PathBuf,
    repeat_count: usize,
    verbose: bool
) -> Result<(), AppError> {
    let initial_gamestate: KnownGameState<15, 15> =
        load_partial_gamestate_from_file(&gamestate_file_path)?
            .try_into()
            .or(Err(AppError::GameStateHadUnknownUnits))?;

    let repeat_count = repeat_count.max(1);
    let mut results = Vec::with_capacity(repeat_count);
    for idx in 0..repeat_count {
        let auto_demystify_result = auto_demystify(
            PseudoPartialGameState::new(initial_gamestate.clone()),
            verbose
        );

        let prefix = if repeat_count > 1 {
            println!("Demystification test #{} results:", idx + 1);
            "\t"
        } else {
            ""
        };

        println!("{}{} step(s)", prefix, auto_demystify_result.step_count);
        println!("{}{} reset(s)", prefix, auto_demystify_result.reset_count);
        println!(
            "{}{:?} search duration",
            prefix, auto_demystify_result.total_demystification_time
        );
        println!(
            "{}{:?} max single-step duration",
            prefix, auto_demystify_result.max_demystification_time
        );
        println!(
            "{}{} pour(s)",
            prefix, auto_demystify_result.total_pour_count
        );
        println!(
            "{}final state solvable: {}",
            prefix,
            if auto_demystify_result.current_state_solvable {
                "yes"
            } else {
                "no"
            }
        );

        results.push(auto_demystify_result);
        if repeat_count > 1 {
            println!();
        }
    }

    if results.len() > 1 {
        //calculate aggregate stats
        let final_solvable_count = results.iter().filter(|x| x.current_state_solvable).count();
        let avg_reset_count =
            (results.iter().map(|x| x.reset_count).sum::<usize>() as f64) / (results.len() as f64);

        let avg_step_count =
            (results.iter().map(|x| x.step_count).sum::<usize>() as f64) / (results.len() as f64);

        let avg_pour_count = (results.iter().map(|x| x.total_pour_count).sum::<usize>() as f64)
            / (results.len() as f64);

        let avg_time = results
            .iter()
            .map(|x| x.total_demystification_time)
            .sum::<Duration>()
            .div_f64(results.len() as f64);
        let avg_max_time = results
            .iter()
            .map(|x| x.max_demystification_time)
            .sum::<Duration>()
            .div_f64(results.len() as f64);
        let overall_max_time = results.iter().map(|x| x.max_demystification_time).max();

        // print aggregate stats
        println!("Averages:");
        println!("\t{:.2} step(s)", avg_step_count,);
        println!("\t{:.2} reset(s)", avg_reset_count);
        println!("\t{:?} search duration", avg_time);
        print!("\t{:?} max single-step duration", avg_max_time);
        if let Some(overall_max_time) = overall_max_time {
            println!(" (overall max seen: {:?})", overall_max_time);
        } else {
            println!();
        }
        println!("\t{:.2} pour(s)", avg_pour_count);
        println!(
            "\t{} of {} final states solvable ({:.2}%)",
            final_solvable_count,
            results.len(),
            (((final_solvable_count as f64) / (results.len() as f64)) * 100.0)
        );
    }

    Ok(())
}
