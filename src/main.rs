use colorgetter::{
    bottle,
    bottle::Bottle,
    colored_water::ColoredWaterUnit,
    gamestate::{GameState, ValidPour}
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
            bottle!([Maroon, Maroon], 4),
        ]
    };
    let mut ostream = stdout();
    base_gamestate.queue_display(&mut ostream)?;
    ostream.flush()?;

    println!("All valid pours:");
    for pour in base_gamestate.iter_pours() {
        println!("{}", pour);
    }

    let pour = ValidPour::try_new(&base_gamestate, 0, 1).expect("Pour failed to be created");
    println!("Selected pour: {}", pour);
    let new_gamestate = pour.apply();

    println!("Gamestate after pour:");
    new_gamestate.queue_display(&mut ostream)?;
    ostream.flush()?;
    println!("All valid pours for new gamestate:");
    for pour in new_gamestate.iter_pours() {
        println!("{}", pour);
    }

    println!("All colors:");
    let color_sampler = GameState {
        bottles: vec![
            bottle!(Red),
            bottle!(Maroon),
            bottle!(Lime),
            bottle!(Green),
            bottle!(Aqua),
            bottle!(Blue),
            bottle!(Yellow),
            bottle!(Orange),
            bottle!(Pink),
            bottle!(Tan),
            bottle!(Brown),
        ]
    };

    color_sampler.queue_display(&mut ostream)?;
    ostream.flush()?;
    Ok(())
}
