use colorgetter::{
    solution::Solution,
    ui::{Ui, UiCreationError, UiRunError}
};

use std::time::Instant;

fn main() -> Result<(), UiCreationError> {
    const MAX_DEPTH: u8 = 0;
    const USE_THREADING: bool = false;
    let ui = Ui::try_new()?;

    match ui.setup_menu_loop::<15, 15>() {
        Err(UiRunError::IOError(e)) => Err(e.into()),
        Err(UiRunError::ExitRequest) => Ok(()),
        Ok(gs) => {
            let start = Instant::now();

            let solution = if USE_THREADING {
                Solution::try_new_threaded(&gs, MAX_DEPTH)
            } else {
                Solution::try_new(&gs, MAX_DEPTH)
            };

            if let Some(solution) = solution {
                let end = Instant::now();
                let duration = end.duration_since(start);
                match ui.solution_viewer_loop(&solution) {
                    Err(UiRunError::IOError(e)) => {
                        return Err(e.into());
                    }
                    Err(UiRunError::ExitRequest) => {
                        return Ok(());
                    }
                    Ok(()) => ()
                };
                drop(ui);
                println!(
                    "{} pour solution found in {:?}",
                    solution.get_pours().len(),
                    duration
                );
            } else {
                drop(ui);
                println!("No solution found")
            }
            Ok(())
        }
    }
}
