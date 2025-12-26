use std::path::PathBuf;

use clap::Parser;
use colorgetter::ui::{Ui, UiError};

fn main() -> Result<(), UiError> {
    let args = Args::parse();

    let ui = Ui::try_new()?;

    let gs = ui.setup_menu_loop(args.gamestate_file.as_deref())?;
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
