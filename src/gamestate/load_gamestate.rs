//! Definition for [GameState] loading from file

use std::{fs, io, path::Path};

use super::GameState;

pub fn load_gamestate_from_file<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    file_path: &Path
) -> Result<GameState<MAX_BCOUNT, B_MAX_CAP>, GameStateLoadError> {
    let file_content = fs::read(file_path)?;
    let gs = serde_json::from_slice(&file_content)?;

    Ok(gs)
}

/// Reasons loading a [GameState] from a file may fail
#[derive(Debug)]
pub enum GameStateLoadError {
    /// Encountered IO error when reading file
    IOError(io::Error),

    /// Encountered error during deserialization
    DeserializeError(serde_json::Error)
}

impl From<io::Error> for GameStateLoadError {
    fn from(value: io::Error) -> Self {
        GameStateLoadError::IOError(value)
    }
}

impl From<serde_json::Error> for GameStateLoadError {
    fn from(value: serde_json::Error) -> Self {
        GameStateLoadError::DeserializeError(value)
    }
}
