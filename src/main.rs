use colorgetter::{
    bottle, bottle::Bottle, colored_water::ColoredWaterUnit, gamestate::GameState,
    solution::Solution
};
use std::{
    io::{self, stdin, stdout, Read, Write},
    num::NonZeroUsize
};

use std::time::Instant;

fn main() -> io::Result<()> {
    println!("Base gamestate:");
    const SIMPLE_GS: bool = true;
    const MAX_DEPTH: u8 = if SIMPLE_GS { 21 } else { 41 };
    const USE_THREADING: bool = true;
    let base_gamestate: GameState<14, 4> = if !SIMPLE_GS {
        [
            bottle!([Blue, Brown, Lime, Blue], 4),
            bottle!([Blue, Yellow, Purple, Maroon], 4),
            bottle!([Orange, Red, Purple, Maroon], 4),
            bottle!([Orange, Red, Pink, Green], 4),
            bottle!([Green, Maroon, Tan, Green], 4),
            bottle!([Lime, Yellow, Pink, Pink], 4),
            bottle!([Aqua, Tan, Purple, Lime], 4),
            // second row
            bottle!([Tan, Aqua, Pink, Brown], 4),
            bottle!([Yellow, Brown, Aqua, Orange], 4),
            bottle!([Maroon, Red, Red, Aqua], 4),
            bottle!([Orange, Brown, Blue, Purple], 4),
            bottle!([Green, Yellow, Tan, Lime], 4),
            bottle!([], 4),
            bottle!([], 4)
        ]
        .into()
    } else {
        [
            bottle!([Orange, Maroon, Yellow, Tan], 4),
            bottle!([Aqua, Maroon, Pink, Pink], 4),
            bottle!([Orange, Aqua, Orange, Green], 4),
            bottle!([Tan, Aqua, Yellow, Aqua], 4),
            bottle!([Pink, Green, Tan, Yellow], 4),
            // second row
            bottle!([Orange, Pink, Maroon, Yellow], 4),
            bottle!([Maroon, Green, Green, Tan], 4),
            bottle!([], 4),
            bottle!([], 4)
        ][..]
            .try_into()
            .unwrap()
    };
    let mut ostream = stdout();
    base_gamestate.queue_display_rows(&mut ostream, NonZeroUsize::new(2).unwrap())?;
    ostream.flush()?;

    println!("Finding solution...");
    let start = Instant::now();

    let solution = if USE_THREADING {
        Solution::try_new_threaded(&base_gamestate, MAX_DEPTH)
    } else {
        Solution::try_new(&base_gamestate, MAX_DEPTH)
    };
    if let Some(solution) = solution {
        let end = Instant::now();
        let duration = end.duration_since(start);
        println!(
            "{} pour solution found in {:?}",
            solution.get_pours().len(),
            duration
        );
        let mut working_gamestate = base_gamestate.clone();
        for pour in solution.get_pours() {
            println!("{}", pour);
            let as_valid = pour
                .try_into_valid(&working_gamestate)
                .expect("Pour from solution wasn't valid?");
            working_gamestate = as_valid.apply();
            working_gamestate.queue_display_rows(&mut ostream, NonZeroUsize::new(2).unwrap())?;
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
