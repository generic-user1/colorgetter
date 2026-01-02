use super::{UiRunError, HIGHLIGHTED_STYLE};
use crate::{
    bottle::{Bottle, PartialBottle},
    colored_water::{PartialColoredWaterIter, RevPartialColoredWaterIter},
    gamestate::{GameStateDisplay, PartialGameState}
};

mod save_menu;
use save_menu::{save_menu_loop, SaveError};

use crossterm::{
    cursor::{MoveDown, MoveRight, MoveToColumn},
    event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{ContentStyle, Print, PrintStyledContent, StyledContent},
    QueueableCommand
};
use heapless;
use std::{
    io::{self, ErrorKind},
    num::NonZeroUsize
};

/// Represents the state of the setup menu
pub(super) struct SetupMenuState<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    pub gs: PartialGameState<MAX_BCOUNT, B_MAX_CAP>,
    pub should_exit: bool,
    c_state: SetupCursorState,
    file_saved_path: Option<Result<String, SaveError>>
}

/// Represents the cursor position in the setup menu
#[derive(PartialEq, Eq)]
enum SetupCursorState {
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

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> SetupMenuState<MAX_BCOUNT, B_MAX_CAP> {
    pub fn new(initial_gamestate: Option<PartialGameState<MAX_BCOUNT, B_MAX_CAP>>) -> Self {
        SetupMenuState {
            c_state: if initial_gamestate.is_none() {
                SetupCursorState::Count
            } else {
                SetupCursorState::Solve
            },
            gs: initial_gamestate.unwrap_or_else(|| PartialGameState {
                bottles: heapless::Vec::new()
            }),
            should_exit: false,
            file_saved_path: None
        }
    }

    pub fn queue_display<T: QueueableCommand>(&self, ostream: &mut T) -> io::Result<()> {
        ostream
            .queue(MoveDown(1))?
            .queue(MoveToColumn(0))?
            .queue(Print("Number of bottles: "))?;
        let bcount_style = if self.c_state == SetupCursorState::Count {
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
                SetupCursorState::Capacity { b_idx } => Some((b_idx, None)),
                SetupCursorState::Content { b_idx, c_idx } => Some((b_idx, Some(c_idx))),
                _ => None
            },
            None
        )?;
        ostream.queue(MoveDown(1))?;

        let solve_prompt_style = if self.c_state == SetupCursorState::Solve {
            HIGHLIGHTED_STYLE
        } else {
            ContentStyle::new()
        };
        ostream.queue(PrintStyledContent(StyledContent::new(
            solve_prompt_style,
            "Confirm and Solve"
        )))?;

        ostream.queue(MoveRight(4))?;

        let save_prompt_style = if self.c_state == SetupCursorState::Save {
            HIGHLIGHTED_STYLE
        } else {
            ContentStyle::new()
        };
        let save_prompt_text = match self.file_saved_path.as_ref() {
            Some(Ok(x)) => format!("Saved to \"{}\"", x),
            Some(Err(e)) => match e {
                SaveError::IOError(e) => match e.kind() {
                    ErrorKind::AlreadyExists => {
                        "Failed to save file due to given file path already being in use".to_owned()
                    }
                    _ => format!("Failed to save file due to IOError: {:?}", e)
                },
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
                            SetupCursorState::Capacity { b_idx } => {
                                let new_b_idx = b_idx + 1;
                                if new_b_idx < self.gs.bottles.len() {
                                    self.c_state = SetupCursorState::Capacity { b_idx: new_b_idx };
                                }
                            }
                            SetupCursorState::Content { b_idx, c_idx } => {
                                //we need to know two things: is there a 'next' bottle, and is our current c_idx in its bounds?
                                let new_b_idx = b_idx + 1;
                                if let Some(bottle) = self.gs.bottles.get(new_b_idx) {
                                    let new_c_idx = c_idx.min(
                                        bottle.get_top_content_idx().map(|i| i + 1).unwrap_or(0)
                                    );
                                    self.c_state = SetupCursorState::Content {
                                        b_idx: new_b_idx,
                                        c_idx: new_c_idx
                                    };
                                }
                            }
                            _ => ()
                        }
                    } else {
                        match self.c_state {
                            SetupCursorState::Count => {
                                //add a bottle if there's room, do nothing if there isn't
                                let _ = self.gs.bottles.push(PartialBottle::try_new(4, 0).unwrap());
                            }
                            SetupCursorState::Capacity { b_idx } => {
                                //increment selected bottle if right is pressed while editing capacity,
                                //even when shift isn't held
                                let new_b_idx = b_idx + 1;
                                if new_b_idx < self.gs.bottles.len() {
                                    self.c_state = SetupCursorState::Capacity { b_idx: new_b_idx };
                                }
                            }
                            SetupCursorState::Content { b_idx, c_idx } => {
                                //change color of selected unit
                                if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                                    //iterate through PartialColoredWaterUnits forwards until we can successfully set a new color
                                    let mut color_iter =
                                        PartialColoredWaterIter(bottle.sample_content_at(c_idx));
                                    loop {
                                        let color_to_use = color_iter.next();
                                        if bottle.try_set_color(c_idx, color_to_use).is_ok() {
                                            break;
                                        }
                                    }
                                }
                            }
                            SetupCursorState::Solve => {
                                self.c_state = SetupCursorState::Save;
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
                            SetupCursorState::Capacity { b_idx } => {
                                let new_b_idx = b_idx.saturating_sub(1);
                                self.c_state = SetupCursorState::Capacity { b_idx: new_b_idx };
                            }
                            SetupCursorState::Content { b_idx, c_idx } => {
                                let new_b_idx = b_idx.saturating_sub(1);
                                if let Some(bottle) = self.gs.bottles.get(new_b_idx) {
                                    let new_c_idx = c_idx.min(
                                        bottle.get_top_content_idx().map(|i| i + 1).unwrap_or(0)
                                    );
                                    self.c_state = SetupCursorState::Content {
                                        b_idx: new_b_idx,
                                        c_idx: new_c_idx
                                    };
                                }
                            }
                            _ => ()
                        }
                    } else {
                        match self.c_state {
                            SetupCursorState::Count => {
                                //remove a bottle if there is one to remove, do nothing if there isn't
                                self.gs.bottles.pop();
                            }
                            SetupCursorState::Capacity { b_idx } => {
                                //decrement selected bottle if left is pressed while editing capacity,
                                //even when shift isn't held
                                let new_b_idx = b_idx.saturating_sub(1);
                                self.c_state = SetupCursorState::Capacity { b_idx: new_b_idx };
                            }
                            SetupCursorState::Content { b_idx, c_idx } => {
                                //change color of selected unit
                                if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                                    //iterate through PartialColoredWaterUnits backwards until we can successfully set a new color
                                    let mut color_iter =
                                        RevPartialColoredWaterIter(bottle.sample_content_at(c_idx));
                                    loop {
                                        let color_to_use = color_iter.next();
                                        if bottle.try_set_color(c_idx, color_to_use).is_ok() {
                                            break;
                                        }
                                    }
                                }
                            }
                            SetupCursorState::Save => {
                                self.c_state = SetupCursorState::Solve;
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
                        SetupCursorState::Capacity { b_idx } => {
                            if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                                let _ = bottle.resize_in_place(bottle.get_capacity() + 1);
                            }
                        }
                        SetupCursorState::Content { b_idx, c_idx } => {
                            //increment the selected color, ensuring we don't go out of capacity bounds
                            //and that our current color isn't empty (so we don't allow empty spaces between two colors)
                            if let Some(bottle) = self.gs.bottles.get(b_idx) {
                                let new_c_idx = c_idx + 1;
                                if new_c_idx < bottle.get_capacity()
                                    && c_idx
                                        < bottle.get_top_content_idx().map(|i| i + 1).unwrap_or(0)
                                {
                                    self.c_state = SetupCursorState::Content {
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
                        SetupCursorState::Capacity { b_idx } => {
                            if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                                //don't allow 0 capacity; a 0 capacity doesn't cause any serious problem but does look weird
                                let new_capacity = bottle.get_capacity().saturating_sub(1);
                                if new_capacity >= 1 {
                                    let _ = bottle
                                        .resize_in_place(bottle.get_capacity().saturating_sub(1));
                                }
                            }
                        }
                        SetupCursorState::Content { b_idx, c_idx } => {
                            self.c_state = SetupCursorState::Content {
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
                    SetupCursorState::Count => {
                        self.c_state = if self.gs.bottles.is_empty() {
                            SetupCursorState::Solve
                        } else {
                            SetupCursorState::Capacity { b_idx: 0 }
                        };
                    }
                    SetupCursorState::Capacity { b_idx } => {
                        self.c_state = SetupCursorState::Content { b_idx, c_idx: 0 };
                    }
                    SetupCursorState::Content { .. } => {
                        self.c_state = SetupCursorState::Solve;
                    }
                    SetupCursorState::Solve => {
                        self.should_exit = true;
                    }
                    SetupCursorState::Save => {
                        self.file_saved_path = save_menu_loop(&self.gs)?;
                    }
                },
                KeyEvent {
                    code: KeyCode::Esc,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => match self.c_state {
                    SetupCursorState::Count => {
                        return Err(UiRunError::ExitRequest);
                    }
                    SetupCursorState::Capacity { .. } => {
                        self.c_state = SetupCursorState::Count;
                    }
                    SetupCursorState::Content { b_idx, .. } => {
                        self.c_state = SetupCursorState::Capacity { b_idx };
                    }
                    SetupCursorState::Solve => {
                        self.c_state = if self.gs.bottles.is_empty() {
                            SetupCursorState::Count
                        } else {
                            SetupCursorState::Content { b_idx: 0, c_idx: 0 }
                        };
                    }
                    SetupCursorState::Save => {
                        self.c_state = SetupCursorState::Save;
                    }
                },
                _ => ()
            }
        }

        Ok(())
    }
}
