//! Implementation of a user interface for setting up [GameState]s

use std::{
    io::{self, stdout, Write},
    marker::PhantomData,
    sync::atomic::{AtomicBool, Ordering}
};

use core::ops::Drop;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen
    },
    QueueableCommand
};

use crate::{gamestate::GameState, solution::Solution};

static UI_EXISTS: AtomicBool = AtomicBool::new(false);

mod setup_menu;
use setup_menu::MenuState;

mod solution_viewer;
use solution_viewer::SolutionViewerState;

/// A struct that represents the user interface. Create an instance of it to set up the UI,
/// use the associated functions to make the UI work, and drop the instance to destroy the UI
/// and return the terminal back to normal.
///
/// Only one instance of Ui is allowed to exist at a time
#[derive(Debug)]
pub struct Ui {
    _phantom: PhantomData<()>
}

impl Ui {
    /// Try to create a new instance of Ui (that is, perform setup for the user interface). This will fail if another instance of Ui already exists.
    pub fn try_new() -> Result<Self, UiCreationError> {
        let ui_exists = UI_EXISTS.load(Ordering::SeqCst);
        if ui_exists {
            Err(UiCreationError::UiAlreadyExists)
        } else {
            UI_EXISTS.store(true, Ordering::SeqCst);
            Self::setup_ui()?;
            Ok(Ui {
                _phantom: PhantomData
            })
        }
    }

    /// Runs a loop that displays the menu for setting up a [GameState](crate::gamestate::GameState) to be solved.
    pub fn setup_menu_loop<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
        &self
    ) -> Result<GameState<MAX_BCOUNT, B_MAX_CAP>, UiRunError> {
        let mut state = MenuState::new();
        loop {
            let mut out = stdout();
            out.queue(Clear(ClearType::All))?.queue(MoveTo(0, 0))?;
            state.queue_display(&mut out)?;
            out.flush()?;
            state.handle_event(event::read()?)?;
            if state.should_exit {
                break Ok(state.gs);
            }
        }
    }

    /// Runs a loop that displays the viewer for a [Solution](crate::solution::Solution)
    pub fn solution_viewer_loop<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
        &self,
        solution: &Solution<MAX_BCOUNT, B_MAX_CAP>
    ) -> Result<(), UiRunError> {
        let mut state = SolutionViewerState::new(solution);
        loop {
            let mut out = stdout();
            out.queue(Clear(ClearType::All))?.queue(MoveTo(0, 0))?;
            state.queue_display(&mut out)?;
            out.flush()?;
            state.handle_event(event::read()?)?;
            if state.should_exit {
                break Ok(());
            }
        }
    }

    /// perform tasks for setting up the Ui. Called by [Ui::try_new]
    fn setup_ui() -> io::Result<()> {
        enable_raw_mode()?;
        stdout()
            .queue(EnterAlternateScreen)?
            .queue(Clear(ClearType::All))?
            .queue(Hide)?
            .flush()?;
        Ok(())
    }

    /// perform tasks for tearing down the Ui. Called by Ui's custom [core::ops::Drop] implementation
    fn teardown_ui() -> io::Result<()> {
        stdout().queue(LeaveAlternateScreen)?.queue(Show)?.flush()?;
        disable_raw_mode()?;
        Ok(())
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        Self::teardown_ui().expect("Failed to tear down UI when dropping instance of Ui");
        UI_EXISTS.store(false, Ordering::SeqCst);
    }
}

/// Reasons creating a [Ui] may fail
#[derive(Debug)]
pub enum UiCreationError {
    /// An instance of [Ui] already exists and we can't create another
    UiAlreadyExists,

    /// An IO error prevented the setup from working correctly
    IOError(io::Error)
}

impl From<io::Error> for UiCreationError {
    fn from(value: io::Error) -> Self {
        UiCreationError::IOError(value)
    }
}

/// Reasons why running the a portion of the [Ui] may end unexpectedly
#[derive(Debug)]
pub enum UiRunError {
    /// An IO error prevented the [Ui] from working correctly
    IOError(io::Error),

    /// The user requested to exit the program using CTRL + C or ESC
    ExitRequest
}

impl From<io::Error> for UiRunError {
    fn from(value: io::Error) -> Self {
        UiRunError::IOError(value)
    }
}
