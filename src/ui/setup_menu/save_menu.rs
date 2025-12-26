use super::{UiRunError, HIGHLIGHTED_STYLE};
use crate::gamestate::GameState;
use crossterm::{
    cursor::{MoveDown, MoveTo, MoveToColumn},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{Attribute, Color, ContentStyle, Print, PrintStyledContent, StyledContent},
    terminal::{Clear, ClearType},
    QueueableCommand
};

use std::{
    fs::File,
    io::{self, stdout, Write}
};

/// Runs a loop that displays the menu for saving a [GameState].
/// Returns the file path saved to, or None if the menu was exited without saving.
pub(super) fn save_menu_loop(
    gs: &GameState
) -> Result<Option<Result<String, SaveError>>, UiRunError> {
    let mut state = SaveMenuState::new(gs);
    loop {
        let mut out = stdout();
        out.queue(Clear(ClearType::All))?.queue(MoveTo(0, 0))?;
        state.queue_display(&mut out)?;
        out.flush()?;
        let exit_reason = state.handle_event(event::read()?)?;
        if let Some(exit_reason) = exit_reason {
            match exit_reason {
                SaveMenuExitReason::SaveAndExit => {
                    break Ok(Some(state.save_gamestate()));
                }
                SaveMenuExitReason::Exit => {
                    break Ok(None);
                }
            }
        }
    }
}

/// Represents the state of the save menu
pub(super) struct SaveMenuState<'a> {
    pub gs: &'a GameState,
    filepath: Vec<char>,
    c_state: SaveCursorState
}

/// Represents the cursor position in the save menu
#[derive(PartialEq, Eq)]
enum SaveCursorState {
    ///Editing the file name; includes the cursor index within the file name
    FileName(usize),

    ///Hovering over the Confirm button
    Confirm
}

impl<'a> SaveMenuState<'a> {
    pub fn new(gs: &'a GameState) -> Self {
        const DEFAULT_PATH: &str = "./saved_gamestate.json";
        SaveMenuState {
            gs,
            filepath: DEFAULT_PATH.chars().collect(),
            c_state: SaveCursorState::FileName(DEFAULT_PATH.len())
        }
    }

    pub fn queue_display<T: QueueableCommand>(&self, ostream: &mut T) -> io::Result<()> {
        ostream
            .queue(MoveDown(1))?
            .queue(MoveToColumn(0))?
            .queue(Print("File path: "))?;

        let fname_str: String = self.filepath.iter().collect();

        match self.c_state {
            SaveCursorState::FileName(idx) => {
                //draw the whole filename highlighted, except don't highlight position
                //of cursor and instead add an underline there

                //first, ensure the cursor position to actually use either points to a character
                //in the filename or points to one character beyond it. in theory, it always should,
                //but doesn't hurt to double-check.
                let cursor_pos = idx.min(self.filepath.len());

                //draw the portion of the filename before the cursor
                ostream.queue(PrintStyledContent(StyledContent::new(
                    HIGHLIGHTED_STYLE,
                    &fname_str.get(..cursor_pos).unwrap_or("")
                )))?;

                //draw the position of the cursor, using a space instead of a
                //char from the string if it's after the end.
                ostream.queue(PrintStyledContent(StyledContent::new(
                    ContentStyle {
                        underline_color: Some(Color::White),
                        attributes: Attribute::Underlined.into(),
                        ..Default::default()
                    },
                    &fname_str.get(cursor_pos..=cursor_pos).unwrap_or(" ")
                )))?;

                //draw the portion of the filename after the cursor
                ostream.queue(PrintStyledContent(StyledContent::new(
                    HIGHLIGHTED_STYLE,
                    &fname_str.get((cursor_pos + 1)..).unwrap_or("")
                )))?;
            }
            SaveCursorState::Confirm => {
                ostream.queue(Print(&fname_str))?;
            }
        }

        ostream.queue(MoveDown(2))?.queue(MoveToColumn(0))?;

        let confirm_style = if self.c_state == SaveCursorState::Confirm {
            HIGHLIGHTED_STYLE
        } else {
            ContentStyle::default()
        };

        ostream.queue(PrintStyledContent(StyledContent::new(
            confirm_style,
            "Confirm Save"
        )))?;

        Ok(())
    }

    pub fn handle_event(&mut self, event: Event) -> Result<Option<SaveMenuExitReason>, UiRunError> {
        if let Event::Key(event) = event {
            match event {
                KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: m,
                    kind: KeyEventKind::Press,
                    ..
                } if m.contains(KeyModifiers::CONTROL) => return Err(UiRunError::ExitRequest),

                KeyEvent {
                    code: KeyCode::Right,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    SaveCursorState::FileName(idx) => {
                        if idx < self.filepath.len() {
                            self.c_state = SaveCursorState::FileName(idx + 1)
                        } else {
                            self.c_state = SaveCursorState::FileName(self.filepath.len())
                        }
                    }
                    SaveCursorState::Confirm => ()
                },

                KeyEvent {
                    code: KeyCode::End,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    SaveCursorState::FileName(_) => {
                        self.c_state = SaveCursorState::FileName(self.filepath.len())
                    }
                    SaveCursorState::Confirm => ()
                },

                KeyEvent {
                    code: KeyCode::Left,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    SaveCursorState::FileName(idx) => {
                        if idx > self.filepath.len() {
                            self.c_state = SaveCursorState::FileName(self.filepath.len())
                        } else if idx > 0 {
                            self.c_state = SaveCursorState::FileName(idx - 1)
                        }
                    }
                    SaveCursorState::Confirm => ()
                },

                KeyEvent {
                    code: KeyCode::Home,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    SaveCursorState::FileName(_) => self.c_state = SaveCursorState::FileName(0),
                    SaveCursorState::Confirm => ()
                },

                KeyEvent {
                    code: KeyCode::Up,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    SaveCursorState::FileName(_) => (),
                    SaveCursorState::Confirm => {
                        self.c_state = SaveCursorState::FileName(self.filepath.len())
                    }
                },

                KeyEvent {
                    code: KeyCode::Down,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    SaveCursorState::FileName(_) => self.c_state = SaveCursorState::Confirm,
                    SaveCursorState::Confirm => ()
                },

                KeyEvent {
                    code: KeyCode::Enter,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    SaveCursorState::FileName(_) => self.c_state = SaveCursorState::Confirm,
                    SaveCursorState::Confirm => {
                        return Ok(Some(SaveMenuExitReason::SaveAndExit));
                    }
                },

                KeyEvent {
                    code: KeyCode::Esc,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => {
                    return Ok(Some(SaveMenuExitReason::Exit));
                }

                KeyEvent {
                    code: KeyCode::Char(c),
                    kind: k,
                    modifiers: m,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    SaveCursorState::FileName(idx) => {
                        let idx_to_use = idx.min(self.filepath.len());
                        self.filepath.insert(idx_to_use, c);
                        self.c_state = SaveCursorState::FileName(idx_to_use + 1);
                    }
                    SaveCursorState::Confirm => ()
                },

                KeyEvent {
                    code: KeyCode::Backspace,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    SaveCursorState::FileName(idx) => {
                        let idx_to_use = idx.min(self.filepath.len()).checked_sub(1);
                        if let Some(idx_to_use) = idx_to_use {
                            self.filepath.remove(idx_to_use);
                            self.c_state = SaveCursorState::FileName(idx_to_use);
                        }
                    }
                    SaveCursorState::Confirm => ()
                },

                KeyEvent {
                    code: KeyCode::Delete,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    SaveCursorState::FileName(idx) => {
                        let idx_to_use = idx.min(self.filepath.len());
                        if idx_to_use < self.filepath.len() {
                            self.filepath.remove(idx_to_use);
                            self.c_state = SaveCursorState::FileName(idx_to_use);
                        }
                    }
                    SaveCursorState::Confirm => ()
                },
                _ => ()
            }
        }

        Ok(None)
    }

    /// Attempts to save current gamestate. Note that this will refuse to write to a file that already exists.
    /// Consumes the save menu, and returns the file path saved to
    pub fn save_gamestate(self) -> Result<String, SaveError> {
        let fname_str: String = self.filepath.into_iter().collect();
        let outfile = File::create_new(&fname_str)?;
        serde_json::to_writer(outfile, &self.gs)?;

        Ok(fname_str)
    }
}

/// Reasons to exit the save menu
pub(crate) enum SaveMenuExitReason {
    /// Save file, then exit menu
    SaveAndExit,

    /// Exit menu without saving
    Exit
}

/// Reasons the save menu may fail
#[derive(Debug)]
pub(crate) enum SaveError {
    /// Couldn't serialize to json
    SerializationError(serde_json::Error),

    /// Couldn't write file/other IO error related to file interaction
    /// Note that this does not include IO errors related to console interaction
    IOError(io::Error)
}
impl From<serde_json::Error> for SaveError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerializationError(value)
    }
}
impl From<io::Error> for SaveError {
    fn from(value: io::Error) -> Self {
        Self::IOError(value)
    }
}
