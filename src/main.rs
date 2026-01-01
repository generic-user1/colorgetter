use std::path::PathBuf;

use clap::Parser;
use colorgetter::ui::{Ui, UiError};

fn main() -> Result<(), UiError> {
    let args = Args::parse();

    let ui = Ui::try_new()?;

    let pgs = ui.setup_menu_loop::<15, 15>(args.gamestate_file.as_deref())?;

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
