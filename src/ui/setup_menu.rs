use super::UiRunError;
use crate::{bottle::Bottle, colored_water::ColoredWaterUnit, gamestate::GameState};
use crossterm::{
    cursor::{MoveDown, MoveToColumn},
    event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{Color, ContentStyle, Print, PrintStyledContent, StyledContent},
    QueueableCommand
};
use heapless;
use std::{io, num::NonZeroUsize};

/// Represents the state of the menu
pub(super) struct MenuState<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    pub gs: GameState<MAX_BCOUNT, B_MAX_CAP>,
    pub c_state: CursorState,
    pub should_exit: bool
}

/// Represents the cursor position in the menu
#[derive(PartialEq, Eq)]
pub(super) enum CursorState {
    /// User is editing the number of Bottles
    Count,

    /// User is editing the capacity of a Bottle.
    /// `b_idx` is the index of the Bottle within the GameState
    Capacity { b_idx: usize },

    /// User is editing the content of a Bottle.
    /// `b_idx` is the index of the Bottle within the GameState,
    /// and `c_idx` is the index of the ColoredWaterUnit slot
    Content { b_idx: usize, c_idx: usize },

    /// User is hovering over the 'Exit' option
    Exit
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> MenuState<MAX_BCOUNT, B_MAX_CAP> {
    pub fn new() -> Self {
        MenuState {
            gs: GameState {
                bottles: heapless::Vec::new()
            },
            c_state: CursorState::Count,
            should_exit: false
        }
    }

    pub fn queue_display<T: QueueableCommand>(&self, ostream: &mut T) -> io::Result<()> {
        ostream
            .queue(MoveDown(1))?
            .queue(MoveToColumn(0))?
            .queue(Print("Number of bottles: "))?;
        let bcount_style = if self.c_state == CursorState::Count {
            ContentStyle {
                background_color: Some(Color::White),
                foreground_color: Some(Color::Black),
                ..Default::default()
            }
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

        let exit_prompt_style = if self.c_state == CursorState::Exit {
            ContentStyle {
                background_color: Some(Color::White),
                foreground_color: Some(Color::Black),
                ..Default::default()
            }
        } else {
            ContentStyle::new()
        };
        ostream.queue(PrintStyledContent(StyledContent::new(
            exit_prompt_style,
            "Save and Solve"
        )))?;

        Ok(())
    }

    pub fn handle_event(&mut self, event: Event) -> Result<(), UiRunError> {
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
                            CursorState::Exit
                        } else {
                            CursorState::Capacity { b_idx: 0 }
                        };
                    }
                    CursorState::Capacity { b_idx } => {
                        self.c_state = CursorState::Content { b_idx, c_idx: 0 };
                    }
                    CursorState::Content { .. } => {
                        self.c_state = CursorState::Exit;
                    }
                    CursorState::Exit => {
                        self.should_exit = true;
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
                    CursorState::Exit => {
                        self.c_state = if self.gs.bottles.is_empty() {
                            CursorState::Count
                        } else {
                            CursorState::Content { b_idx: 0, c_idx: 0 }
                        };
                    }
                },
                _ => ()
            }
        }

        Ok(())
    }
}
