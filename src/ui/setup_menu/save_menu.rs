use super::{UiRunError, HIGHLIGHTED_STYLE};
use crossterm::{
    cursor::{MoveDown, MoveTo, MoveToColumn},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{Attribute, Color, ContentStyle, Print, PrintStyledContent, StyledContent},
    terminal::{Clear, ClearType},
    QueueableCommand
};
use serde::Serialize;

use std::{
    fs::File,
    io::{self, stdout, ErrorKind, Write}
};

/// Runs a loop that displays the menu for saving a [KnownGameState](crate::gamestate::KnownGameState) or [PartialGameState](crate::gamestate::PartialGameState).
///
/// Technically, can be used for saving anything that implements [Serialize], though it's specifically meant for (and used for) game states.
/// Returns the file path saved to, or None if the menu was exited without saving.
pub(crate) fn save_menu_loop<T: Serialize>(gs: &T) -> Result<Option<String>, UiRunError> {
    let mut state = SaveMenuState::new(gs);
    loop {
        let mut out = stdout();
        out.queue(Clear(ClearType::All))?.queue(MoveTo(0, 0))?;
        state.queue_display(&mut out)?;
        out.flush()?;
        match state.handle_event(event::read()?)? {
            SaveMenuEventResult::SaveAndExit => match state.save_gamestate() {
                Ok(filepath) => {
                    break Ok(Some(filepath));
                }
                Err(e) => {
                    state.update_err_msg(e);
                }
            },
            SaveMenuEventResult::Exit => {
                break Ok(None);
            }
            SaveMenuEventResult::Nothing => ()
        }
    }
}

/// Represents the state of the save menu
pub(super) struct SaveMenuState<'a, T: Serialize> {
    pub gs: &'a T,
    filepath: Vec<char>,
    c_state: SaveCursorState,
    last_err_msg: String
}

/// Represents the cursor position in the save menu
#[derive(PartialEq, Eq)]
enum SaveCursorState {
    ///Editing the file name; includes the cursor index within the file name
    FileName(usize),

    ///Hovering over the Confirm button
    Confirm
}

impl<'a, T: Serialize> SaveMenuState<'a, T> {
    pub fn new(gs: &'a T) -> Self {
        const DEFAULT_PATH: &str = "./saved_gamestate.json";
        SaveMenuState {
            gs,
            filepath: DEFAULT_PATH.chars().collect(),
            c_state: SaveCursorState::FileName(DEFAULT_PATH.len()),
            last_err_msg: "".to_string()
        }
    }

    pub fn update_err_msg(&mut self, err: SaveError) {
        self.last_err_msg = err.to_message()
    }
    pub fn clear_err_msg(&mut self) {
        self.last_err_msg = "".to_string()
    }

    pub fn queue_display<U: QueueableCommand>(&self, ostream: &mut U) -> io::Result<()> {
        ostream.queue(MoveDown(1))?.queue(MoveToColumn(0))?;

        if !self.last_err_msg.is_empty() {
            ostream
                .queue(Print(format!("Error: {}", self.last_err_msg)))?
                .queue(MoveDown(2))?
                .queue(MoveToColumn(0))?;
        }
        ostream.queue(Print("File path: "))?;

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

    pub fn handle_event(&mut self, event: Event) -> Result<SaveMenuEventResult, UiRunError> {
        if let Event::Key(event) = event {
            if event.kind == KeyEventKind::Press && !self.last_err_msg.is_empty() {
                self.clear_err_msg();
            }
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
                        return Ok(SaveMenuEventResult::SaveAndExit);
                    }
                },

                KeyEvent {
                    code: KeyCode::Esc,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => {
                    return Ok(SaveMenuEventResult::Exit);
                }

                KeyEvent {
                    code: KeyCode::Char(c),
                    kind: k,
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

        Ok(SaveMenuEventResult::Nothing)
    }

    /// Attempts to save current gamestate. Note that this will refuse to write to a file that already exists.
    ///
    /// Returns the file path saved to if saving was successful
    pub fn save_gamestate(&self) -> Result<String, SaveError> {
        let fname_str: String = self.filepath.iter().collect();
        let outfile = File::create_new(&fname_str)?;
        serde_json::to_writer(outfile, &self.gs)?;

        Ok(fname_str)
    }
}

/// What to do after handling an event in the save menu
pub(crate) enum SaveMenuEventResult {
    /// Save file, then exit menu
    SaveAndExit,

    /// Exit menu without saving
    Exit,

    /// Do nothing
    Nothing
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
impl SaveError {
    /// Returns a human-readable error message representing this SaveError
    pub fn to_message(&self) -> String {
        match self {
            SaveError::IOError(e) => match e.kind() {
                ErrorKind::AlreadyExists => {
                    "Failed to save file due to given file path already being in use".to_owned()
                }
                _ => format!("Failed to save file due to IOError: {:?}", e)
            },
            SaveError::SerializationError(e) => {
                format!("Failed to save file due to SerializationError: {:?}", e)
            }
        }
    }
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
