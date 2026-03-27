use super::{UiRunError, HIGHLIGHTED_STYLE};
use crate::{
    bottle::{Bottle, PartialBottle},
    colored_water::{PartialColoredWaterIter, RevPartialColoredWaterIter},
    gamestate::{GameState, PartialGameState}
};

mod save_menu;
pub(super) use save_menu::save_menu_loop;

use crossterm::{
    cursor::{MoveDown, MoveRight, MoveToColumn},
    event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{ContentStyle, Print, PrintStyledContent, StyledContent},
    terminal::{Clear, ClearType},
    QueueableCommand
};
use heapless;
use std::{io, num::NonZeroUsize};

/// Data used for the 'specific bottle' mode of the setup menu
struct SpecificBottleData {
    /// The index of the bottle within the game state
    pub bottle_idx: usize,

    /// The number of unknown color units in the specific bottle
    /// when the setup menu first started; used to restrict what units can be edited.
    pub original_unknown_count: usize
}
/// Represents the state of the setup menu
pub(super) struct SetupMenuState<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    pub gs: PartialGameState<MAX_BCOUNT, B_MAX_CAP>,
    c_state: SetupCursorState,
    file_saved_path: Option<String>,
    specific_bottle_data: Option<SpecificBottleData>,
    needs_screen_clear: bool
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
    pub fn new(
        initial_gamestate: Option<PartialGameState<MAX_BCOUNT, B_MAX_CAP>>,
        specific_bottle_idx: Option<usize>
    ) -> Self {
        // set up specific bottle data. if no specific index was given, or no initial gamestate was given,
        // or both were given but the index pointed to a nonexistant bottle, or the specified bottle has no unknown units,
        // then our specific_bottle_data ends up being None.
        let specific_bottle_data = specific_bottle_idx.and_then(|idx| {
            initial_gamestate
                .as_ref()
                .and_then(|gs| gs.bottles.get(idx))
                .map(|b| SpecificBottleData {
                    bottle_idx: idx,
                    original_unknown_count: b.get_unknown_count()
                })
                .filter(|d| d.original_unknown_count > 0)
        });

        let c_state = if let Some(specific_bottle_data) = &specific_bottle_data {
            // if we have specific bottle data, our cursor state should default to the specified bottle,
            // pointing to the topmost unknown unit
            SetupCursorState::Content {
                b_idx: specific_bottle_data.bottle_idx,
                c_idx: specific_bottle_data
                    .original_unknown_count
                    .saturating_sub(1)
            }
        } else if initial_gamestate.is_some() {
            // if we don't have specific bottle data but we do have an initial gamestate,
            // cursor should default to the solve button
            SetupCursorState::Solve
        } else {
            // if we don't have an initial gamestate, cursor should default to the bottle count editor
            SetupCursorState::Count
        };

        SetupMenuState {
            c_state,
            gs: initial_gamestate.unwrap_or_else(|| PartialGameState {
                bottles: heapless::Vec::new()
            }),
            file_saved_path: None,
            specific_bottle_data,
            needs_screen_clear: true
        }
    }

    /// Clear the screen if needed, do nothing if not needed
    ///
    /// Resets the internal flag tracking whether the screen needs to be cleared, so calling
    /// twice in a row will result in the second call always choosing not to clear the screen.
    ///
    /// Returns whether the screen was cleared or not
    pub fn clear_screen_if_needed<T: QueueableCommand>(
        &mut self,
        ostream: &mut T
    ) -> io::Result<bool> {
        Ok(if self.needs_screen_clear {
            ostream.queue(Clear(ClearType::All))?;
            self.needs_screen_clear = false;
            true
        } else {
            false
        })
    }

    pub fn queue_display<T: QueueableCommand>(&self, ostream: &mut T) -> io::Result<()> {
        if self.specific_bottle_data.is_none() {
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
        }
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
            if self.specific_bottle_data.is_none() {
                "Confirm and Solve"
            } else {
                "Confirm and Find Next Color"
            }
        )))?;

        ostream.queue(MoveRight(4))?;

        let save_prompt_style = if self.c_state == SetupCursorState::Save {
            HIGHLIGHTED_STYLE
        } else {
            ContentStyle::new()
        };
        let save_prompt_text = match self.file_saved_path.as_ref() {
            Some(saved_path) => format!("Saved to \"{}\"", saved_path),
            None => "Save to File".to_owned()
        };
        ostream.queue(PrintStyledContent(StyledContent::new(
            save_prompt_style,
            save_prompt_text
        )))?;

        Ok(())
    }

    /// Returns true if handling this event means we should exit, false if we shouldn't exit and should keep going instead.
    pub fn handle_event(&mut self, event: Event) -> Result<bool, UiRunError> {
        if let Event::Key(event) = event {
            if event.kind == KeyEventKind::Press && self.file_saved_path.is_some() {
                self.file_saved_path = None;
                self.needs_screen_clear = true;
            }

            let retval = match event {
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
                        self.handle_cursor_shift_right()
                    } else {
                        self.handle_cursor_right()
                    }
                }
                KeyEvent {
                    code: KeyCode::Left,
                    modifiers: m,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => {
                    if m.contains(KeyModifiers::SHIFT) {
                        self.handle_cursor_shift_left()
                    } else {
                        self.handle_cursor_left()
                    }
                }
                KeyEvent {
                    code: KeyCode::Up,
                    kind: k,
                    ..
                } if (k == KeyEventKind::Press || k == KeyEventKind::Repeat) => {
                    self.handle_cursor_up()
                }
                KeyEvent {
                    code: KeyCode::Down,
                    kind: k,
                    ..
                } if (k == KeyEventKind::Press || k == KeyEventKind::Repeat) => {
                    self.handle_cursor_down()
                }
                KeyEvent {
                    code: KeyCode::Enter,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => self.handle_enter(),
                KeyEvent {
                    code: KeyCode::Esc,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => self.handle_esc(),
                _ => Ok(false)
            };

            // if a specific bottle is set, ensure cursor is in one of its allowed states
            // after handling the event. if the cursor is in an invalid state, reset it.
            //
            // we do this *after* handling an event and not before because
            // handling an event with a cursor in an invalid state does nothing,
            // but resetting the cursor to a valid state before handling it would lead to unexpected behavior
            // since the state the user saw when they pressed a button and the state when the event got processed would differ.
            if let Some(b_data) = &self.specific_bottle_data {
                match self.c_state {
                    SetupCursorState::Capacity { .. } | SetupCursorState::Count => {
                        self.c_state = SetupCursorState::Content {
                            b_idx: b_data.bottle_idx,
                            c_idx: b_data.original_unknown_count.saturating_sub(1)
                        };
                    }
                    SetupCursorState::Content { b_idx, c_idx }
                        if b_idx != b_data.bottle_idx || c_idx >= b_data.original_unknown_count =>
                    {
                        self.c_state = SetupCursorState::Content {
                            b_idx: b_data.bottle_idx,
                            c_idx: b_data.original_unknown_count.saturating_sub(1)
                        }
                    }
                    _ => ()
                }
            }
            return retval;
        }

        //default return value if the event wasn't a KeyEvent is `Ok(false)`
        Ok(false)
    }

    /// Handle the right arrow being pressed (without shift)
    ///
    /// When called from [SetupMenuState::handle_event], return value of this
    /// function should be used as the return value of the `handle_event` function overall
    fn handle_cursor_right(&mut self) -> Result<bool, UiRunError> {
        match self.c_state {
            SetupCursorState::Count => {
                //ensure no specific bottle is set
                if self.specific_bottle_data.is_none() {
                    //add a bottle if there's room, do nothing if there isn't
                    let added_bottle = self
                        .gs
                        .bottles
                        .push(PartialBottle::try_new(4, 0).unwrap())
                        .is_ok();
                    if added_bottle {
                        self.needs_screen_clear = true;
                    }
                }
            }
            SetupCursorState::Capacity { b_idx } => {
                //ensure no specific bottle is set
                if self.specific_bottle_data.is_none() {
                    //increment selected bottle if right is pressed while editing capacity,
                    //even when shift isn't held
                    let new_b_idx = b_idx + 1;
                    if new_b_idx < self.gs.bottles.len() {
                        self.c_state = SetupCursorState::Capacity { b_idx: new_b_idx };
                    }
                }
            }
            SetupCursorState::Content { b_idx, c_idx } => {
                //if a specific bottle is set, and the b_idx and c_idx are not where we expect them to be,
                //bail out before changing anything
                if let Some(b_data) = &self.specific_bottle_data {
                    if b_idx != b_data.bottle_idx || c_idx >= b_data.original_unknown_count {
                        return Ok(false);
                    }
                }

                //change color of selected unit
                if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                    //iterate through PartialColoredWaterUnits forwards until we can successfully set a new color
                    let mut color_iter = PartialColoredWaterIter(bottle.sample_content_at(c_idx));
                    loop {
                        let color_to_use = color_iter.next();
                        //disallow "empty" if a specific bottle is set
                        if self.specific_bottle_data.is_some() && color_to_use.is_none() {
                            continue;
                        }

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
        Ok(false)
    }

    /// Handle the left arrow being pressed (without shift)
    ///
    /// When called from [SetupMenuState::handle_event], return value of this
    /// function should be used as the return value of the `handle_event` function overall
    fn handle_cursor_left(&mut self) -> Result<bool, UiRunError> {
        match self.c_state {
            SetupCursorState::Count => {
                //ensure no specific bottle is set
                if self.specific_bottle_data.is_none() {
                    //remove a bottle if there is one to remove, do nothing if there isn't
                    let removed_bottle = self.gs.bottles.pop().is_some();
                    if removed_bottle {
                        self.needs_screen_clear = true;
                    }
                }
            }
            SetupCursorState::Capacity { b_idx } => {
                //ensure no specific bottle is set
                if self.specific_bottle_data.is_none() {
                    //decrement selected bottle if left is pressed while editing capacity,
                    //even when shift isn't held
                    let new_b_idx = b_idx.saturating_sub(1);
                    self.c_state = SetupCursorState::Capacity { b_idx: new_b_idx };
                }
            }
            SetupCursorState::Content { b_idx, c_idx } => {
                //if a specific bottle is set, and the b_idx and c_idx are not where we expect them to be,
                //bail out before changing anything
                if let Some(b_data) = &self.specific_bottle_data {
                    if b_idx != b_data.bottle_idx || c_idx >= b_data.original_unknown_count {
                        return Ok(false);
                    }
                }

                //change color of selected unit
                if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                    //iterate through PartialColoredWaterUnits backwards until we can successfully set a new color
                    let mut color_iter =
                        RevPartialColoredWaterIter(bottle.sample_content_at(c_idx));
                    loop {
                        let color_to_use = color_iter.next();
                        //disallow "empty" if a specific bottle is set
                        if self.specific_bottle_data.is_some() && color_to_use.is_none() {
                            continue;
                        }

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
        Ok(false)
    }

    /// Handle the right arrow being pressed with shift
    ///
    /// If pressing the right arrow with shift would do nothing, the behavior
    /// of pressing the right arrow without shift will be used
    ///
    /// When called from [SetupMenuState::handle_event], return value of this
    /// function should be used as the return value of the `handle_event` function overall
    fn handle_cursor_shift_right(&mut self) -> Result<bool, UiRunError> {
        //if there is a specific bottle set, run normal cursor right behavior immediately
        if self.specific_bottle_data.is_some() {
            return self.handle_cursor_right();
        }

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
                    let new_c_idx =
                        c_idx.min(bottle.get_top_content_idx().map(|i| i + 1).unwrap_or(0));
                    self.c_state = SetupCursorState::Content {
                        b_idx: new_b_idx,
                        c_idx: new_c_idx
                    };
                }
            }
            _ => return self.handle_cursor_right()
        }
        Ok(false)
    }

    /// Handle the left arrow being pressed with shift
    ///
    /// If pressing the left arrow with shift would do nothing, the behavior
    /// of pressing the left arrow without shift will be used
    ///
    /// When called from [SetupMenuState::handle_event], return value of this
    /// function should be used as the return value of the `handle_event` function overall
    fn handle_cursor_shift_left(&mut self) -> Result<bool, UiRunError> {
        //if there is a specific bottle set, run normal cursor leftt behavior immediately
        if self.specific_bottle_data.is_some() {
            return self.handle_cursor_left();
        }

        //decrement selected bottle
        match self.c_state {
            SetupCursorState::Capacity { b_idx } => {
                let new_b_idx = b_idx.saturating_sub(1);
                self.c_state = SetupCursorState::Capacity { b_idx: new_b_idx };
            }
            SetupCursorState::Content { b_idx, c_idx } => {
                let new_b_idx = b_idx.saturating_sub(1);
                if let Some(bottle) = self.gs.bottles.get(new_b_idx) {
                    let new_c_idx =
                        c_idx.min(bottle.get_top_content_idx().map(|i| i + 1).unwrap_or(0));
                    self.c_state = SetupCursorState::Content {
                        b_idx: new_b_idx,
                        c_idx: new_c_idx
                    };
                }
            }
            _ => return self.handle_cursor_left()
        }
        Ok(false)
    }

    /// Handle the up arrow being pressed
    ///
    /// When called from [SetupMenuState::handle_event], return value of this
    /// function should be used as the return value of the `handle_event` function overall
    fn handle_cursor_up(&mut self) -> Result<bool, UiRunError> {
        match self.c_state {
            SetupCursorState::Capacity { b_idx } => {
                //ensure no specific bottle is set
                if self.specific_bottle_data.is_none() {
                    if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                        let added_cap = bottle.resize_in_place(bottle.capacity() + 1).is_ok();
                        if added_cap {
                            self.needs_screen_clear = true;
                        }
                    }
                }
            }
            SetupCursorState::Content { b_idx, c_idx } => {
                //if a specific bottle is set, and the b_idx and c_idx are not where we expect them to be,
                //bail out here. note that we'll also bail out if c_idx is pointing to the top unknown unit,
                //making it impossible to select a unit above the top unknown
                if let Some(b_data) = &self.specific_bottle_data {
                    if b_idx != b_data.bottle_idx
                        || c_idx >= (b_data.original_unknown_count.saturating_sub(1))
                    {
                        return Ok(false);
                    }
                }

                //increment the selected color, ensuring we don't go out of capacity bounds
                //and that our current color isn't empty (so we don't allow empty spaces between two colors)
                if let Some(bottle) = self.gs.bottles.get(b_idx) {
                    let new_c_idx = c_idx + 1;
                    if new_c_idx < bottle.capacity()
                        && c_idx < bottle.get_top_content_idx().map(|i| i + 1).unwrap_or(0)
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
        Ok(false)
    }

    /// Handle the down arrow being pressed
    ///
    /// When called from [SetupMenuState::handle_event], return value of this
    /// function should be used as the return value of the `handle_event` function overall
    fn handle_cursor_down(&mut self) -> Result<bool, UiRunError> {
        match self.c_state {
            SetupCursorState::Capacity { b_idx } => {
                //ensure no specific bottle is set
                if self.specific_bottle_data.is_none() {
                    if let Some(bottle) = self.gs.bottles.get_mut(b_idx) {
                        //don't allow 0 capacity; a 0 capacity doesn't cause any serious problem but does look weird
                        let new_capacity = bottle.capacity().saturating_sub(1);
                        if new_capacity >= 1 {
                            let removed_cap = bottle
                                .resize_in_place(bottle.capacity().saturating_sub(1))
                                .is_ok();
                            if removed_cap {
                                self.needs_screen_clear = true;
                            }
                        }
                    }
                }
            }
            SetupCursorState::Content { b_idx, c_idx } => {
                //if a specific bottle is set, and the b_idx and c_idx are not where we expect them to be,
                //bail out before editing anything
                if let Some(b_data) = &self.specific_bottle_data {
                    if b_idx != b_data.bottle_idx || c_idx >= b_data.original_unknown_count {
                        return Ok(false);
                    }
                }
                self.c_state = SetupCursorState::Content {
                    b_idx,
                    c_idx: c_idx.saturating_sub(1)
                };
            }
            _ => ()
        }
        Ok(false)
    }

    /// Handle the enter key being pressed
    ///
    /// When called from [SetupMenuState::handle_event], return value of this
    /// function should be used as the return value of the `handle_event` function overall
    fn handle_enter(&mut self) -> Result<bool, UiRunError> {
        match self.c_state {
            SetupCursorState::Count => {
                //ensure no specific bottle is set
                if self.specific_bottle_data.is_none() {
                    self.c_state = if self.gs.bottles.is_empty() {
                        SetupCursorState::Solve
                    } else {
                        SetupCursorState::Capacity { b_idx: 0 }
                    };
                }
            }
            SetupCursorState::Capacity { b_idx } => {
                //ensure no specific bottle is set
                if self.specific_bottle_data.is_none() {
                    self.c_state = SetupCursorState::Content { b_idx, c_idx: 0 };
                }
            }
            SetupCursorState::Content { .. } => {
                self.c_state = SetupCursorState::Solve;
            }
            SetupCursorState::Solve => {
                return Ok(true);
            }
            SetupCursorState::Save => {
                self.file_saved_path = save_menu_loop(&self.gs)?;
                self.needs_screen_clear = true;
            }
        }
        Ok(false)
    }

    /// Handle the escape key being pressed
    ///
    /// When called from [SetupMenuState::handle_event], return value of this
    /// function should be used as the return value of the `handle_event` function overall
    fn handle_esc(&mut self) -> Result<bool, UiRunError> {
        match self.c_state {
            SetupCursorState::Count => {
                //ensure no specific bottle is set
                if self.specific_bottle_data.is_none() {
                    return Err(UiRunError::ExitRequest);
                }
            }
            SetupCursorState::Capacity { .. } => {
                //ensure no specific bottle is set
                if self.specific_bottle_data.is_none() {
                    self.c_state = SetupCursorState::Count;
                }
            }
            SetupCursorState::Content { b_idx, .. } => {
                //ensure no specific bottle is set
                if self.specific_bottle_data.is_none() {
                    self.c_state = SetupCursorState::Capacity { b_idx };
                }
            }
            SetupCursorState::Solve => {
                self.c_state = if let Some(b_data) = &self.specific_bottle_data {
                    SetupCursorState::Content {
                        b_idx: b_data.bottle_idx,
                        c_idx: b_data.original_unknown_count.saturating_sub(1)
                    }
                } else if self.gs.bottles.is_empty() {
                    SetupCursorState::Count
                } else {
                    SetupCursorState::Content { b_idx: 0, c_idx: 0 }
                };
            }
            SetupCursorState::Save => {
                self.c_state = SetupCursorState::Save;
            }
        }
        Ok(false)
    }
}
