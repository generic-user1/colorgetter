use std::path::PathBuf;

use clap::{Parser, Subcommand};
use colorgetter::{
    gamestate::{load_partial_gamestate_from_file, GameStateLoadError},
    ui::{Ui, UiCreationError, UiRunError}
};

mod demystification_test;
use demystification_test::demystification_test;

fn main() -> Result<(), AppError> {
    let args = Args::parse();
    match args.action.unwrap_or_default() {
        Action::Solve {
            gamestate_file,
            save_demystified
        } => solve(gamestate_file, save_demystified),
        Action::TestDemystification {
            gamestate_file,
            verbose,
            num_repeats
        } => demystification_test(gamestate_file, num_repeats, verbose)
    }
}

/// Run the solver Ui
fn solve(gamestate_file_path: Option<PathBuf>, save_demystified: bool) -> Result<(), AppError> {
    let ui = Ui::try_new()?;

    let initial_game_state = if let Some(game_state_file_path) = gamestate_file_path {
        Some(load_partial_gamestate_from_file(&game_state_file_path)?)
    } else {
        None
    };

    let pgs = ui.setup_menu_loop::<15, 15>(initial_game_state)?;

    let demystified = ui.demystifier_loop(pgs)?;
    if save_demystified {
        ui.save_demystified(&demystified)?;
    }
    let solution = ui.demystified_result_solution_finding_loop(&demystified)?;
    if let Some(solution) = solution {
        ui.solution_viewer_loop(&solution)?;
    }
    Ok(())
}

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    action: Option<Action>
}

#[derive(Subcommand, Clone)]
enum Action {
    /// Solve a given game state
    Solve {
        /// Path to a saved game state to solve. The game state may have a mix of known and unknown colors.
        /// The game state editor will be initialized to this state if it is provided. If not provided,
        /// the editor will be initialized to an empty state.
        #[arg(short, long, value_name = "FILE_PATH")]
        gamestate_file: Option<PathBuf>,

        /// Whether to save a copy of the initial gamestate after demystifying.
        /// If this is present, a save dialog will be shown immediately after demystifying,
        /// but before the actual solution is found and run.
        ///
        /// This will likely be removed as an option in the future.
        #[arg(short, long)]
        save_demystified: bool
    },

    /// Test and print statistics about demystification
    TestDemystification {
        /// Path to a saved game state to test demystification with.
        /// The game state must only have known colors.
        /// The state that gets demystified will be this state with all colors
        /// not on top of their respective bottles set to unknown.
        #[arg(short, long, value_name = "FILE_PATH")]
        gamestate_file: PathBuf,

        /// Whether to print information about demystification test as each test is running
        /// or only the summary after each test finishes.
        #[arg(short, long)]
        verbose: bool,

        /// Number of times to repeat the demystification test with the same state.
        /// If less than 1, will be interpreted as 1.
        #[arg(short, long, default_value_t = 1)]
        num_repeats: usize
    }
}
impl Default for Action {
    fn default() -> Self {
        Action::Solve {
            gamestate_file: None,
            save_demystified: false
        }
    }
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
