use std::path::PathBuf;

use clap::Parser;
use colorgetter::{
    gamestate::{load_partial_gamestate_from_file, GameStateLoadError, PseudoPartialGameState},
    solution::{auto_demystify, Solution},
    ui::{Ui, UiCreationError, UiRunError}
};

fn main() -> Result<(), AppError> {
    let args = Args::parse();

    if args.test_demystification {
        let gamestate_file_path = args.gamestate_file.unwrap();
        demystification_test(gamestate_file_path)
    } else {
        solve(args.gamestate_file)
    }
}

/// Run the solver Ui
fn solve(gamestate_file_path: Option<PathBuf>) -> Result<(), AppError> {
    let ui = Ui::try_new()?;

    let initial_game_state = if let Some(game_state_file_path) = gamestate_file_path {
        Some(load_partial_gamestate_from_file(&game_state_file_path)?)
    } else {
        None
    };

    let pgs = ui.setup_menu_loop::<15, 15>(initial_game_state)?;

    let demystified = ui.demystifier_loop(pgs)?;

    let solution = ui.demystified_result_solution_finding_loop(&demystified)?;
    if let Some(solution) = solution {
        ui.solution_viewer_loop(&solution)?;
    }
    Ok(())
}

/// Run the demystification test
fn demystification_test(gamestate_file_path: PathBuf) -> Result<(), AppError> {
    let initial_gamestate = load_partial_gamestate_from_file::<15, 15>(&gamestate_file_path)?
        .try_into()
        .or(Err(AppError::GameStateHadUnknownUnits))?;

    let auto_demystify_result =
        auto_demystify(PseudoPartialGameState::new(initial_gamestate), true);

    let is_current_solvable = Solution::try_new(&auto_demystify_result.current_state, 0).is_some();
    println!(
        "Demystification took {} step(s) and required {} reset(s)",
        auto_demystify_result.step_count, auto_demystify_result.reset_count
    );
    println!(
        "Spent {:?} finding demystification next-steps",
        auto_demystify_result.total_demystification_time
    );
    println!(
        "{} pour(s) used as part of demystification",
        auto_demystify_result.total_pour_count
    );
    if is_current_solvable {
        println!("Final state is solvable!")
    } else {
        println!(
            "Final state is not solvable, requiring 1 additional reset for a total of {} reset(s)",
            auto_demystify_result.reset_count + 1
        );
    }
    Ok(())
}

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Path to a saved game state to solve. The game state editor will be initialized
    /// to this state if it is provided. If not provided, the editor will be initialized
    /// to an empty state.
    #[arg(short, long, value_name = "FILE_PATH")]
    gamestate_file: Option<PathBuf>,

    /// Activates demystification testing. If this is set, the "-g"/"--gamestate-file"
    /// must be used, and must point to a gamestate file with no unknown colors. This will
    /// convert the gamestate to a partial gamestate, automatically demystify and solve it,
    /// and print statistics on how long the process took and how many resets were needed.
    ///
    /// This will likely be removed as an option in the future.
    #[arg(short, long, requires = "gamestate_file")]
    test_demystification: bool
}

/// Reasons the application may fail with an error
#[derive(Debug)]
enum AppError {
    /// Demystification testing was requested, but provided file had a [PartialGameState](colorgetter::gamestate::PartialGameState)
    #[allow(dead_code)]
    GameStateHadUnknownUnits,

    /// An initial [PartialGameState](colorgetter::gamestate::PartialGameState) file was provided, but couldn't be loaded
    #[allow(dead_code)]
    GameStateLoad(GameStateLoadError),

    /// Encountered a [UiCreationError] while creating the [Ui]
    #[allow(dead_code)]
    UiCreation(UiCreationError),

    /// Encountered a [UiRunError] while running some portion of the [Ui]
    #[allow(dead_code)]
    UiRun(UiRunError)
}

impl From<GameStateLoadError> for AppError {
    fn from(value: GameStateLoadError) -> Self {
        Self::GameStateLoad(value)
    }
}
impl From<UiCreationError> for AppError {
    fn from(value: UiCreationError) -> Self {
        Self::UiCreation(value)
    }
}
impl From<UiRunError> for AppError {
    fn from(value: UiRunError) -> Self {
        Self::UiRun(value)
    }
}
