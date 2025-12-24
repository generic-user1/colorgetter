use super::UiRunError;
use crate::{bottle::Bottle, colored_water::ColoredWaterUnit, gamestate::GameState};
use crossterm::{
    cursor::{MoveDown, MoveRight, MoveToColumn},
    event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{Attributes, Color, ContentStyle, Print, PrintStyledContent, StyledContent},
    QueueableCommand
};
use heapless;
use std::{fs::File, io, num::NonZeroUsize, path::PathBuf};

/// Represents the state of the setup menu
pub(super) struct MenuState<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    pub gs: GameState<MAX_BCOUNT, B_MAX_CAP>,
    pub should_exit: bool,
    c_state: CursorState,
    file_saved_path: Option<Result<PathBuf, SaveError>>
}

/// Represents the cursor position in the menu
#[derive(PartialEq, Eq)]
enum CursorState {
    /// User is editing the number of Bottles
    Count,

    /// User is editing the capacity of a Bottle.
    /// `b_idx` is the index of the Bottle within the GameState
    Capacity { b_idx: usize },

    /// User is editing the content of a Bottle.
    /// `b_idx` is the index of the Bottle within the GameState,
    /// and `c_idx` is the index of the ColoredWaterUnit slot
    Content { b_idx: usize, c_idx: usize },

    /// User is hovering over the 'Solve' option
    Solve,

    /// User is hovering over the 'Save' option
    Save
}

/// Style for a highlighted item; white background, black text, no underline, no attributes.
const HIGHLIGHTED_STYLE: ContentStyle = ContentStyle {
    background_color: Some(Color::White),
    foreground_color: Some(Color::Black),
    underline_color: None,
    attributes: Attributes::none()
};

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> MenuState<MAX_BCOUNT, B_MAX_CAP> {
    pub fn new() -> Self {
        MenuState {
            gs: GameState {
                bottles: heapless::Vec::new()
            },
            c_state: CursorState::Count,
            should_exit: false,
            file_saved_path: None
        }
    }

    pub fn queue_display<T: QueueableCommand>(&self, ostream: &mut T) -> io::Result<()> {
        ostream
            .queue(MoveDown(1))?
            .queue(MoveToColumn(0))?
            .queue(Print("Number of bottles: "))?;
        let bcount_style = if self.c_state == CursorState::Count {
            HIGHLIGHTED_STYLE
        } else {
            ContentStyle::new()
        };
        ostream.queue(PrintStyledContent(StyledContent::new(
            bcount_style,
            format!("{:>3}", self.gs.bottles.len())
        )))?;
        ostream.queue(MoveDown(2))?.queue(MoveToColumn(0))?;
        self.gs.queue_display_rows(
            ostream,
            NonZeroUsize::new(2).unwrap(),
            match self.c_state {
                CursorState::Capacity { b_idx } => Some((b_idx, None)),
                CursorState::Content { b_idx, c_idx } => Some((b_idx, Some(c_idx))),
                _ => None
            },
            None
        )?;
        ostream.queue(MoveDown(1))?;

        let solve_prompt_style = if self.c_state == CursorState::Solve {
            HIGHLIGHTED_STYLE
        } else {
            ContentStyle::new()
        };
        ostream.queue(PrintStyledContent(StyledContent::new(
            solve_prompt_style,
            "Confirm and Solve"
        )))?;

        ostream.queue(MoveRight(4))?;

        let save_prompt_style = if self.c_state == CursorState::Save {
            HIGHLIGHTED_STYLE
        } else {
            ContentStyle::new()
        };
        let save_prompt_text = match self.file_saved_path.as_ref() {
            Some(Ok(x)) => format!(
                "Saved to \"{}\"",
                x.to_str().unwrap_or("<unknown file path>")
            ),
            Some(Err(e)) => match e {
                SaveError::NoAvailableFilename => {
                    "Failed to save file: could not find file name not already in use".to_owned()
                }
                SaveError::IOError(e) => format!("Failed to save file due to IOError: {:?}", e),
                SaveError::SerializationError(e) => {
                    format!("Failed to save file due to SerializationError: {:?}", e)
                }
            },
            None => "Save to File".to_owned()
        };
        ostream.queue(PrintStyledContent(StyledContent::new(
            save_prompt_style,
            save_prompt_text
        )))?;

        Ok(())
    }

    pub fn handle_event(&mut self, event: Event) -> Result<(), UiRunError> {
        if let Event::Key(event) = event {
            if event.kind == KeyEventKind::Press && self.file_saved_path.is_some() {
                self.file_saved_path = None;
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
                    modifiers: m,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => {
                    if m.contains(KeyModifiers::SHIFT) {
                        //increment selected bottle if the cursor is over bottles and there is a 'next' bottle to select
                        match self.c_state {
                            CursorState::Capacity { b_idx } => {
                                let new_b_idx = b_idx + 1;
                                if new_b_idx < self.gs.bottles.len() {
                                    self.c_state = CursorState::Capacity { b_idx: new_b_idx };
                                }
                            }
                            CursorState::Content { b_idx, c_idx } => {
                                //we need to know two things: is there a 'next' bottle, and is our current c_idx in its bounds?
                                let new_b_idx = b_idx + 1;
                                if let Some(bottle) = self.gs.bottles.get(new_b_idx) {
                                    let new_c_idx = c_idx.min(bottle.get_content().len());
                                    self.c_state = CursorState::Content {
                                        b_idx: new_b_idx,
                                        c_idx: new_c_idx
                                    };
                                }
                            }
                            _ => ()
                        }
                    } else {
                        match self.c_state {
                            CursorState::Count => {
                                //add a bottle if there's room, do nothing if there isn't
                                let _ = self.gs.bottles.push(Bottle::try_new(4).unwrap());
                            }
                            CursorState::Capacity { b_idx } => {
                                //increment selected bottle if right is pressed while editing capacity,
                                //even when shift isn't held
                                let new_b_idx = b_idx + 1;
                                if new_b_idx < self.gs.bottles.len() {
                                    self.c_state = CursorState::Capacity { b_idx: new_b_idx };
                                }
                            }
                            CursorState::Content { b_idx, c_idx } => {
                                //change color of selected unit
                                if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                                    let next_color = if let Some(current_color) =
                                        bottle.get_content().get(c_idx)
                                    {
                                        current_color.next()
                                    } else {
                                        Some(ColoredWaterUnit::first())
                                    };
                                    let _ = bottle.try_set_color(c_idx, next_color);
                                }
                            }
                            CursorState::Solve => {
                                self.c_state = CursorState::Save;
                            }
                            _ => ()
                        }
                    }
                }
                KeyEvent {
                    code: KeyCode::Left,
                    modifiers: m,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => {
                    if m.contains(KeyModifiers::SHIFT) {
                        //decrement selected bottle
                        match self.c_state {
                            CursorState::Capacity { b_idx } => {
                                let new_b_idx = b_idx.saturating_sub(1);
                                self.c_state = CursorState::Capacity { b_idx: new_b_idx };
                            }
                            CursorState::Content { b_idx, c_idx } => {
                                let new_b_idx = b_idx.saturating_sub(1);
                                if let Some(bottle) = self.gs.bottles.get(new_b_idx) {
                                    let new_c_idx = c_idx.min(bottle.get_content().len());
                                    self.c_state = CursorState::Content {
                                        b_idx: new_b_idx,
                                        c_idx: new_c_idx
                                    };
                                }
                            }
                            _ => ()
                        }
                    } else {
                        match self.c_state {
                            CursorState::Count => {
                                //remove a bottle if there is one to remove, do nothing if there isn't
                                self.gs.bottles.pop();
                            }
                            CursorState::Capacity { b_idx } => {
                                //decrement selected bottle if left is pressed while editing capacity,
                                //even when shift isn't held
                                let new_b_idx = b_idx.saturating_sub(1);
                                self.c_state = CursorState::Capacity { b_idx: new_b_idx };
                            }
                            CursorState::Content { b_idx, c_idx } => {
                                //change color of selected unit
                                if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                                    let prev_color = if let Some(current_color) =
                                        bottle.get_content().get(c_idx)
                                    {
                                        current_color.prev()
                                    } else {
                                        Some(ColoredWaterUnit::last())
                                    };
                                    let _ = bottle.try_set_color(c_idx, prev_color);
                                }
                            }
                            CursorState::Save => {
                                self.c_state = CursorState::Solve;
                            }
                            _ => ()
                        }
                    }
                }

                KeyEvent {
                    code: KeyCode::Up,
                    kind: k,
                    ..
                } if (k == KeyEventKind::Press || k == KeyEventKind::Repeat) => {
                    match self.c_state {
                        CursorState::Capacity { b_idx } => {
                            if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                                let _ = bottle.resize_in_place(bottle.get_capacity() + 1);
                            }
                        }
                        CursorState::Content { b_idx, c_idx } => {
                            //increment the selected color, ensuring we don't go out of capacity bounds
                            //and that our current color isn't empty (so we don't allow empty spaces between two colors)
                            if let Some(bottle) = self.gs.bottles.get(b_idx) {
                                let new_c_idx = c_idx + 1;
                                if new_c_idx < bottle.get_capacity()
                                    && c_idx < bottle.get_content().len()
                                {
                                    self.c_state = CursorState::Content {
                                        b_idx,
                                        c_idx: new_c_idx
                                    };
                                }
                            }
                        }
                        _ => ()
                    }
                }
                KeyEvent {
                    code: KeyCode::Down,
                    kind: k,
                    ..
                } if (k == KeyEventKind::Press || k == KeyEventKind::Repeat) => {
                    match self.c_state {
                        CursorState::Capacity { b_idx } => {
                            if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                                //don't allow 0 capacity; a 0 capacity doesn't cause any serious problem but does look weird
                                let new_capacity = bottle.get_capacity().saturating_sub(1);
                                if new_capacity >= 1 {
                                    let _ = bottle
                                        .resize_in_place(bottle.get_capacity().saturating_sub(1));
                                }
                            }
                        }
                        CursorState::Content { b_idx, c_idx } => {
                            self.c_state = CursorState::Content {
                                b_idx,
                                c_idx: c_idx.saturating_sub(1)
                            };
                        }
                        _ => ()
                    }
                }
                KeyEvent {
                    code: KeyCode::Enter,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    CursorState::Count => {
                        self.c_state = if self.gs.bottles.is_empty() {
                            CursorState::Solve
                        } else {
                            CursorState::Capacity { b_idx: 0 }
                        };
                    }
                    CursorState::Capacity { b_idx } => {
                        self.c_state = CursorState::Content { b_idx, c_idx: 0 };
                    }
                    CursorState::Content { .. } => {
                        self.c_state = CursorState::Solve;
                    }
                    CursorState::Solve => {
                        self.should_exit = true;
                    }
                    CursorState::Save => {
                        self.file_saved_path = Some(self.save_gamestate());
                    }
                },
                KeyEvent {
                    code: KeyCode::Esc,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    CursorState::Count => {
                        return Err(UiRunError::ExitRequest);
                    }
                    CursorState::Capacity { .. } => {
                        self.c_state = CursorState::Count;
                    }
                    CursorState::Content { b_idx, .. } => {
                        self.c_state = CursorState::Capacity { b_idx };
                    }
                    CursorState::Solve => {
                        self.c_state = if self.gs.bottles.is_empty() {
                            CursorState::Count
                        } else {
                            CursorState::Content { b_idx: 0, c_idx: 0 }
                        };
                    }
                    CursorState::Save => {
                        self.c_state = CursorState::Save;
                    }
                },
                _ => ()
            }
        }

        Ok(())
    }

    ///Saves current gamestate, returning file path saved to
    pub fn save_gamestate(&self) -> Result<PathBuf, SaveError> {
        let base_path = PathBuf::from("./saved_gamestate.json");
        let path_to_use = if base_path.try_exists().unwrap_or(true) {
            let mut num: u16 = 1;
            let mut path_with_num = PathBuf::from(format!("./saved_gamestate_{}.json", num));
            while path_with_num.try_exists().unwrap_or(true) {
                num = num.checked_add(1).ok_or(SaveError::NoAvailableFilename)?;
                path_with_num = PathBuf::from(format!("./saved_gamestate_{}.json", num));
            }
            path_with_num
        } else {
            base_path
        };

        let outfile = File::create_new(&path_to_use)?;
        serde_json::to_writer(outfile, &self.gs)?;

        Ok(path_to_use)
    }
}

/// Reasons saving a gamestate may fail
#[derive(Debug)]
pub(crate) enum SaveError {
    /// Couldn't serialize to json
    SerializationError(serde_json::Error),

    /// Couldn't write file/other IO error
    IOError(io::Error),

    /// Couldn't find a file name that wasn't already in use
    NoAvailableFilename
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
