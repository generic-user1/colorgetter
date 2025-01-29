use colorgetter::{
    bottle,
    bottle::Bottle,
    colored_water::ColoredWaterUnit,
    gamestate::{GameState, Pour}
};
use std::io::{self, stdout, Write};

fn main() -> io::Result<()> {
    println!("Base gamestate:");
    let base_gamestate = GameState {
        bottles: vec![
            bottle!([Red, Red, Red], 4),
            bottle!([Orange, Green, Red], 4),
            bottle!([Yellow, Maroon], 3),
            bottle!([Brown, Lime, Aqua, Aqua], 4),
        ]
    };
    let mut ostream = stdout();
    base_gamestate.queue_display(&mut ostream)?;
    ostream.flush()?;

    let pour = Pour::try_new(&base_gamestate, 0, 1).expect("Pour failed to be created");
    println!(
        "Pour bottle {} into bottle {}",
        pour.get_source_index(),
        pour.get_dest_index()
    );
    let new_gamestate = pour.apply();

    println!("Gamestate after pour:");
    new_gamestate.queue_display(&mut ostream)?;
    ostream.flush()?;
    Ok(())
}
