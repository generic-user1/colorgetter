use colorgetter::{
    solution::Solution,
    ui::{SetupMenuError, Ui, UiCreationError}
};
use std::{
    io::{stdin, stdout, Read, Write},
    num::NonZeroUsize
};

use std::time::Instant;

fn main() -> Result<(), UiCreationError> {
    const MAX_DEPTH: u8 = 0;
    const USE_THREADING: bool = false;
    let ui = Ui::try_new()?;

    match ui.setup_menu_loop::<15, 15>() {
        Err(SetupMenuError::IOError(e)) => Err(e.into()),
        Err(SetupMenuError::ExitRequest) => Ok(()),
        Ok(gs) => {
            drop(ui);
            println!("Base gamestate:");
            let mut ostream = stdout();
            gs.queue_display_rows(&mut ostream, NonZeroUsize::new(2).unwrap(), None)?;
            ostream.flush()?;
            println!("Finding solution...");
            let start = Instant::now();

            let solution = if USE_THREADING {
                Solution::try_new_threaded(&gs, MAX_DEPTH)
            } else {
                Solution::try_new(&gs, MAX_DEPTH)
            };

            if let Some(solution) = solution {
                let end = Instant::now();
                let duration = end.duration_since(start);
                println!(
                    "{} pour solution found in {:?}",
                    solution.get_pours().len(),
                    duration
                );
                let mut working_gamestate = gs.clone();
                for pour in solution.get_pours() {
                    println!("{}", pour);
                    let as_valid = pour
                        .try_into_valid(&working_gamestate)
                        .expect("Pour from solution wasn't valid?");
                    working_gamestate = as_valid.apply();
                    working_gamestate.queue_display_rows(
                        &mut ostream,
                        NonZeroUsize::new(2).unwrap(),
                        None
                    )?;
                    ostream.flush()?;

                    println!("Press Enter to continue...");
                    if cfg!(windows) {
                        // on windows, one enter keypress is two bytes (presumably CR + LF)
                        stdin().read_exact(&mut [0, 0]).unwrap();
                    } else {
                        // assume that one enter keypress is one byte on other platforms
                        stdin().read_exact(&mut [0]).unwrap();
                    }
                }
            } else {
                println!("No solution found")
            }
            Ok(())
        }
    }
}
