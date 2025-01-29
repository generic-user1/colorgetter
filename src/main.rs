use colorgetter::{bottle, bottle::Bottle, colored_water::ColoredWaterUnit, gamestate::GameState};
use std::io::{self, stdout, Write};

fn main() -> io::Result<()> {
    let base_gamestate = GameState {
        bottles: vec![
            bottle!([Red, Red, Red], 4),
            bottle!([Red, Green, Orange], 4),
            bottle!([Yellow, Maroon], 3),
            bottle!([Brown, Lime, Aqua, Aqua], 4),
        ]
    };
    let mut ostream = stdout();
    base_gamestate.queue_display(&mut ostream)?;
    ostream.flush()?;
    Ok(())
}
