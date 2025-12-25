//! Implementation of a user interface for setting up [GameState]s

use std::{
    io::{self, stdout, Write},
    marker::PhantomData,
    sync::Mutex,
    time::Duration
};

use core::ops::Drop;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event,
    style::{Attributes, Color, ContentStyle},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen
    },
    QueueableCommand
};

use crate::{gamestate::GameState, solution::Solution};

static UI_EXISTS: Mutex<bool> = Mutex::new(false);

/// Style for a highlighted item; white background, black text, no underline, no attributes.
const HIGHLIGHTED_STYLE: ContentStyle = ContentStyle {
    background_color: Some(Color::White),
    foreground_color: Some(Color::Black),
    underline_color: None,
    attributes: Attributes::none()
};

mod setup_menu;
use setup_menu::SetupMenuState;

mod solution_wait_screen;
use solution_wait_screen::{GetSolutionError, WaitScreenState};

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
        let mut ui_exists = UI_EXISTS.lock().unwrap();
        if *ui_exists {
            Err(UiCreationError::UiAlreadyExists)
        } else {
            *ui_exists = true;
            Self::setup_ui()?;
            Ok(Ui {
                _phantom: PhantomData
            })
        }
    }

    /// Runs a loop that displays the menu for setting up a [GameState] to be solved.
    pub fn setup_menu_loop<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
        &self
    ) -> Result<GameState<MAX_BCOUNT, B_MAX_CAP>, UiRunError> {
        let mut state = SetupMenuState::new();
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

    /// Runs a loop that handles display and input while the given [GameState] is solved
    /// in the background.
    ///
    /// Returns an [`Option<Solution>`]; if [None], no solution could be found.
    pub fn solution_finding_loop<'a, const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
        &self,
        gamestate_to_solve: &'a GameState<MAX_BCOUNT, B_MAX_CAP>
    ) -> Result<Option<Solution<'a, MAX_BCOUNT, B_MAX_CAP>>, UiRunError> {
        let mut state = WaitScreenState::new(gamestate_to_solve);
        let mut out = stdout();
        loop {
            let is_finished = state.check_finished();
            out.queue(Clear(ClearType::All))?.queue(MoveTo(0, 0))?;
            state.queue_display(&mut out)?;
            out.flush()?;

            // wait to handle an event if we're finished;
            // if we're not finished, handle events only if there are any events to be handled
            if is_finished || event::poll(Duration::from_millis(16))? {
                state.handle_event(event::read()?)?;
            }

            if state.should_exit {
                match state.get_solution() {
                    Ok(solution) => return Ok(Some(solution)),
                    Err(e) => match e {
                        GetSolutionError::NoSolutionFound => return Ok(None),
                        GetSolutionError::NotYetFinished => {
                            panic!("Not yet finished, but should_exit was true")
                        }
                    }
                }
            }
        }
    }

    /// Runs a loop that displays the viewer for a [Solution]
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
        let mut ui_exists = UI_EXISTS.lock().unwrap();
        *ui_exists = false;
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

/// All errors the [Ui] can encounter
#[derive(Debug)]
pub enum UiError {
    /// Encountered a [UiCreationError] while creating the [Ui]
    CreationError(UiCreationError),

    /// Encountered a [UiRunError] while running some portion of the [Ui]
    RunError(UiRunError)
}

impl From<UiCreationError> for UiError {
    fn from(value: UiCreationError) -> Self {
        Self::CreationError(value)
    }
}

impl From<UiRunError> for UiError {
    fn from(value: UiRunError) -> Self {
        Self::RunError(value)
    }
}
