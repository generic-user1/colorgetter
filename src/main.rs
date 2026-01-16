use std::path::PathBuf;

use clap::Parser;
use colorgetter::{
    gamestate::{load_partial_gamestate_from_file, GameStateLoadError},
    ui::{Ui, UiCreationError, UiRunError}
};

fn main() -> Result<(), AppError> {
    let args = Args::parse();

    let ui = Ui::try_new()?;

    let initial_game_state = if let Some(game_state_file_path) = args.gamestate_file {
        Some(load_partial_gamestate_from_file(&game_state_file_path)?)
    } else {
        None
    };

    let pgs = ui.setup_menu_loop::<15, 15>(initial_game_state)?;

    // TODO: for now, we just take the partial gamestate out of the setup menu, attempt to directly convert it to a gamestate,
    // and panic if that fails. This is useful temporarily so that we can test the setup/save menus on partial gamestates without having
    // to properly implement solving partial states, but will need to be fixed soon
    let gs = pgs.try_into().expect("Couldn't directly convert from PartialGameState to GameState. This error will likely be fixed in the future.");

    let solution = ui.solution_finding_loop(&gs)?;
    if let Some(solution) = solution {
        ui.solution_viewer_loop(&solution)?;
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
    gamestate_file: Option<PathBuf>
}

/// Reasons the application may fail with an error
#[derive(Debug)]
enum AppError {
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
