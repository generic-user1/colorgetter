//! Implementation for the state of an entire game

mod gamestate_def;
pub use gamestate_def::GameState;

mod pour_def;
pub use pour_def::{PourError, ValidPour, ValidPourIter};

#[cfg(test)]
mod pour_tests;
