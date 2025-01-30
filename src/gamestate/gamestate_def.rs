use crate::{bottle::Bottle, gamestate::ValidPourIter};
use crossterm::{
    cursor::{MoveDown, MoveLeft, MoveRight},
    style::{ContentStyle, Print, PrintStyledContent, StyledContent},
    QueueableCommand
};
use std::io;

/// The state a particular game is in
///
/// That is, represents what bottles exist and what order they're in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    pub bottles: Vec<Bottle>
}

impl GameState {
    /// Queues the display of this GameState for the given `ostream` (typically [std::io::stdout])
    ///
    /// Does not flush; the caller must call flush on `ostream` in order to actually display the GameState.
    pub fn queue_display<T: QueueableCommand>(&self, ostream: &mut T) -> io::Result<()> {
        // TODO: break GameState into multiple lines in some cases

        const FILLED: char = '█';
        const EMPTY: char = '▒';
        const NOTHING: char = ' ';

        // Cursor movement doesn't appear to work right on Windows, so we use regular characters.
        // Using an alternate screen might make cursor movement work though, so this is left as an option for now.
        const USE_CURSOR: bool = false;

        if let Some(row_count) = self.bottles.iter().map(|b| b.get_capacity()).max() {
            for row_index in (0..row_count).rev() {
                for bottle in &self.bottles {
                    if bottle.get_capacity() < (row_index + 1) {
                        // if capacity is less than row index + 1 (row_index is 0-based so we add 1 for 1-based),
                        // this bottle doesn't have the capacity to reach here, so we print nothing
                        if USE_CURSOR {
                            ostream.queue(MoveRight(1))?;
                        } else {
                            ostream.queue(Print(NOTHING))?;
                        }
                    } else if let Some(color) = bottle.get_content().get(row_index) {
                        let styled_content = StyledContent::new(
                            ContentStyle {
                                foreground_color: Some(color.into()),
                                ..Default::default()
                            },
                            FILLED
                        );
                        ostream.queue(PrintStyledContent(styled_content))?;
                    } else {
                        // bottle has capacity to reach here, but there's no content there;
                        // print empty space
                        ostream.queue(Print(EMPTY))?;
                    }
                    // print an empty space between bottles
                    if USE_CURSOR {
                        ostream.queue(MoveRight(1))?;
                    } else {
                        ostream.queue(Print(NOTHING))?;
                    }
                }
                //move cursor to beginning of next row
                if USE_CURSOR {
                    ostream.queue(MoveLeft(
                        (self.bottles.len() * 2).try_into().unwrap_or(65535)
                    ))?;
                    ostream.queue(MoveDown(1))?;
                } else {
                    ostream.queue(Print('\n'))?;
                }
            }
        }

        Ok(())
    }

    /// Returns whether this GameState represents a finished game
    ///
    /// The game is finished when all bottles are either completely empty or
    /// completely full of a single color
    pub fn is_finished(&self) -> bool {
        for bottle in &self.bottles {
            if !(bottle.is_in_final_state() || bottle.get_content().is_empty()) {
                return false;
            }
        }
        true
    }

    /// Returns an iterator over all valid [Pour](crate::gamestate::Pour)s you could apply to this GameState
    pub fn iter_pours(&self) -> ValidPourIter {
        ValidPourIter::new(self)
    }
}

// trait implementations
impl From<Vec<Bottle>> for GameState {
    fn from(value: Vec<Bottle>) -> Self {
        Self { bottles: value }
    }
}

impl From<&[Bottle]> for GameState {
    fn from(value: &[Bottle]) -> Self {
        Self {
            bottles: value.into()
        }
    }
}

impl From<GameState> for Vec<Bottle> {
    fn from(value: GameState) -> Self {
        value.bottles
    }
}
