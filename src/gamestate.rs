//! Implementation for the state of an entire game and operations that can be performed on it

mod gamestate_def;
pub use gamestate_def::GameState;

mod pour_def;
pub use pour_def::{Pour, PourError, ValidPour, ValidPourIter};

#[cfg(test)]
mod pour_tests;
