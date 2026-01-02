//! Implementation for the state of an entire game and operations that can be performed on it

mod gamestate_def;
pub use gamestate_def::KnownGameState;

mod partial_gamestate;
pub use partial_gamestate::PartialGameState;

mod gamestate_display;
pub use gamestate_display::GameStateDisplay;

mod pour_def;
pub use pour_def::{Pour, PourError, ValidPour, ValidPourIter};

mod load_gamestate;
pub use load_gamestate::{
    load_gamestate_from_file, load_partial_gamestate_from_file, GameStateLoadError
};

#[cfg(test)]
mod pour_tests;
