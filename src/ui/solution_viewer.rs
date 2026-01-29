use crossterm::{
    cursor::{MoveDown, MoveToColumn},
    event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::Print,
    QueueableCommand
};
use std::{io, num::NonZeroUsize, ops::Deref};

use super::UiRunError;
use crate::{
    gamestate::{GameState, Pour, SolvableGameState},
    solution::Solution
};

pub(super) struct SolutionViewerState<'a, T>
where
    T: Deref,
    <T as Deref>::Target: SolvableGameState
{
    /// The [Solution] this SolutionViewerState is meant to display
    ///
    /// Private so we can ensure the Solution never changes, which is important so we can
    /// ensure our current_pour_idx is always valid.
    solution: &'a Solution<T>,

    /// The index of the pour within our solution's valid pours that's currently being displayed.
    ///
    /// Optional because displaying just the initial GameState requires no pours, but index 0 refers to the first pour.
    /// Private because otherwise, we can't guarantee that this index points to a valid pour within the solution
    current_pour_idx: Option<usize>
}

impl<'a, T> SolutionViewerState<'a, T>
where
    T: Deref,
    <T as Deref>::Target: SolvableGameState
{
    pub fn new(solution: &'a Solution<T>) -> SolutionViewerState<'a, T> {
        SolutionViewerState {
            solution,
            current_pour_idx: None
        }
    }

    pub fn queue_display<U: QueueableCommand>(&self, ostream: &mut U) -> io::Result<()> {
        let (displayed_gs, displayed_pour) = self.get_displayed_state_and_pour();
        if let Some(displayed_pour) = displayed_pour {
            ostream.queue(Print(format!(
                "Step {} of {}: Pour from {} to {}",
                self.current_pour_idx.unwrap() + 1,
                self.solution.get_pours().len(),
                displayed_pour.source_bottle_index + 1,
                displayed_pour.dest_bottle_index + 1
            )))?;
        } else {
            ostream.queue(Print("Base Gamestate:"))?;
        }
        ostream.queue(MoveDown(1))?.queue(MoveToColumn(0))?;
        displayed_gs.queue_display_rows(
            ostream,
            NonZeroUsize::new(2).unwrap(),
            None,
            displayed_pour
        )?;
        Ok(())
    }

    /// Returns true if handling this event means we should exit, false if we shouldn't exit and should keep going instead.
    pub fn handle_event(&mut self, event: Event) -> Result<bool, UiRunError> {
        if let Event::Key(event) = event {
            match event {
                KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: m,
                    kind: KeyEventKind::Press,
                    ..
                } if m.contains(KeyModifiers::CONTROL) => return Err(UiRunError::ExitRequest),
                KeyEvent {
                    code: KeyCode::Left,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => {
                    let _ = self.try_dec_current_pour_idx();
                }
                KeyEvent {
                    code: KeyCode::Right,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => {
                    let inc_result = self.try_inc_current_pour_idx();
                    if inc_result.is_err() {
                        return Ok(true);
                    }
                }
                KeyEvent {
                    code: KeyCode::Enter,
                    kind: k,
                    ..
                } if k == KeyEventKind::Press || k == KeyEventKind::Repeat => {
                    let inc_result = self.try_inc_current_pour_idx();
                    if inc_result.is_err() {
                        return Ok(true);
                    }
                }
                _ => ()
            }
        }
        Ok(false)
    }

    /// Returns the currently displayed [GameState] and, if it exists, the [Pour] used to get to the
    /// current [GameState] from the previous one.
    fn get_displayed_state_and_pour(&self) -> (T::Target, Option<&Pour>) {
        if let Some(display_pour_idx) = self.current_pour_idx {
            let mut working_gs = self.solution.get_base_gamestate().clone();
            for (current_pour_idx, current_pour) in self.solution.get_pours().iter().enumerate() {
                let current_valid_pour = current_pour
                    .try_into_valid(&working_gs)
                    .expect("Pour from solution wasn't valid");
                working_gs = current_valid_pour.apply();

                if current_pour_idx >= display_pour_idx {
                    return (working_gs, Some(current_pour));
                }
            }
            panic!("display_pour_idx out of bounds");
        } else {
            (self.solution.get_base_gamestate().clone(), None)
        }
    }

    /// Attempts to set the `current_pour_idx`. Fails if out of bounds.
    fn try_set_current_pour_idx(
        &mut self,
        new_pour_idx: Option<usize>
    ) -> Result<(), PourIdxChangeError> {
        if let Some(new_pour_idx) = new_pour_idx {
            if new_pour_idx < self.solution.get_pours().len() {
                self.current_pour_idx = Some(new_pour_idx);
                Ok(())
            } else {
                Err(PourIdxChangeError::OutOfBounds)
            }
        } else {
            self.current_pour_idx = None;
            Ok(())
        }
    }

    /// Attempts to increment the `current_pour_idx`. Fails if out of bounds
    fn try_inc_current_pour_idx(&mut self) -> Result<(), PourIdxChangeError> {
        if let Some(current_pour_idx) = self.current_pour_idx {
            self.try_set_current_pour_idx(Some(current_pour_idx + 1))
        } else {
            self.try_set_current_pour_idx(Some(0))
        }
    }

    /// Attempts to decrement the `current_pour_idx`. Fails if out of bounds
    fn try_dec_current_pour_idx(&mut self) -> Result<(), PourIdxChangeError> {
        if let Some(current_pour_idx) = self.current_pour_idx {
            self.try_set_current_pour_idx(current_pour_idx.checked_sub(1))
        } else {
            Err(PourIdxChangeError::OutOfBounds)
        }
    }
}

/// Reasons setting the current pour index may fail
enum PourIdxChangeError {
    OutOfBounds
}
