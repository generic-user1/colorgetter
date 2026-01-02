//! Definition for [KnownGameState]/[PartialGameState] loading from file

use std::{fs, io, path::Path};

use super::{KnownGameState, PartialGameState};

/// Load a [PartialGameState] from a JSON file at the given `file_path`
///
/// **Note**: using this function to load a file that was generated from a [KnownGameState] (instead of a [PartialGameState])
/// will succeed, though each [PartialBottle](crate::bottle::PartialBottle)'s `unknown_count` will be set to 0.
pub fn load_partial_gamestate_from_file<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    file_path: &Path
) -> Result<PartialGameState<MAX_BCOUNT, B_MAX_CAP>, GameStateLoadError> {
    let file_content = fs::read(file_path)?;
    let gs = serde_json::from_slice(&file_content)?;

    Ok(gs)
}

/// Load a [KnownGameState] from a JSON file at the given `file_path`
///
/// **Note**: using this function to load a file that was generated from a [PartialGameState] (instead of a [KnownGameState])
/// will succeed, though each [KnownBottle](crate::bottle::KnownBottle) will ignore any `unknown_count`
pub fn load_gamestate_from_file<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    file_path: &Path
) -> Result<KnownGameState<MAX_BCOUNT, B_MAX_CAP>, GameStateLoadError> {
    let file_content = fs::read(file_path)?;
    let gs = serde_json::from_slice(&file_content)?;

    Ok(gs)
}
/// Reasons loading a [KnownGameState] or [PartialGameState] from a file may fail
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
