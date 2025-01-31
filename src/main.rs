use colorgetter::{
    bottle, bottle::Bottle, colored_water::ColoredWaterUnit, gamestate::GameState,
    solution::Solution
};
use std::io::{self, stdin, stdout, Read, Write};

use std::time::Instant;

#[allow(clippy::unused_io_amount)]
fn main() -> io::Result<()> {
    println!("Base gamestate:");
    let base_gamestate = GameState {
        bottles: [
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
        ]
    };
    let mut ostream = stdout();
    base_gamestate.queue_display(&mut ostream)?;
    ostream.flush()?;

    println!("Finding solution...");
    let start = Instant::now();
    if let Some(solution) = Solution::try_new(&base_gamestate) {
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
            working_gamestate.queue_display(&mut ostream)?;
            ostream.flush()?;

            println!("Press Enter to continue...");
            stdin().read(&mut [0]).unwrap();
        }
    } else {
        println!("No solution found")
    }

    Ok(())
}
