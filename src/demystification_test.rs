use std::{path::PathBuf, time::Duration};

use super::AppError;

use colorgetter::{
    gamestate::{load_partial_gamestate_from_file, KnownGameState, PseudoPartialGameState},
    solution::auto_demystify
};

/// Run the demystification test
pub fn demystification_test(
    gamestate_file_paths: Vec<PathBuf>,
    repeat_count: usize,
    verbose: bool
) -> Result<(), AppError> {
    let gamestate_file_count = gamestate_file_paths.len();
    let has_multiple = gamestate_file_count > 1;

    //first, load all gamestate files - this is so that if any of the files are invalid, we error out early,
    //rather than performing the test on some of the files and then failing before all tests can be completed.
    //we do still keep track of each gamestate's path for display purposes
    let mut loaded_gamestates = Vec::with_capacity(gamestate_file_count);
    for gamestate_file_path in gamestate_file_paths {
        let loaded_gamestate: KnownGameState<15, 15> =
            load_partial_gamestate_from_file(&gamestate_file_path)?
                .try_into()
                .or(Err(AppError::GameStateHadUnknownUnits))?;
        loaded_gamestates.push((loaded_gamestate, gamestate_file_path));
    }

    for (gamestate_idx, (loaded_gamestate, loaded_gamestate_path)) in
        loaded_gamestates.into_iter().enumerate()
    {
        if has_multiple {
            if gamestate_idx > 0 {
                //add some extra vertical space between subsequent demystification test results
                println!();
            }
            println!(
                "Demystification test on gamestate file \"{}\" (file {} of {})",
                loaded_gamestate_path.to_string_lossy(),
                gamestate_idx + 1,
                gamestate_file_count
            );
        }

        let repeat_count = repeat_count.max(1);
        let mut results = Vec::with_capacity(repeat_count);
        for idx in 0..repeat_count {
            let auto_demystify_result = auto_demystify(
                PseudoPartialGameState::new(loaded_gamestate.clone()),
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
            let avg_reset_count = (results.iter().map(|x| x.reset_count).sum::<usize>() as f64)
                / (results.len() as f64);

            let avg_step_count = (results.iter().map(|x| x.step_count).sum::<usize>() as f64)
                / (results.len() as f64);

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
    }
    Ok(())
}
