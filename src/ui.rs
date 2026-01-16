//! Implementation of a user interface for setting up [KnownGameState](crate::gamestate::KnownGameState)s/[PartialGameState]s

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

use crate::{
    bottle::{Bottle, BottleSampleResult},
    colored_water::PartialColoredWaterUnit,
    gamestate::{GameState, KnownGameState, PartialGameState, SolvableGameState},
    solution::Solution
};

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

    /// Runs a loop that displays the menu for setting up a [PartialGameState] to be solved.
    ///
    /// `initial_game_state` is an optional [PartialGameState] to start with.
    /// If this is provided, the setup menu will initialize to the provided [PartialGameState];
    /// if not, the initial [PartialGameState] will be empty.
    pub fn setup_menu_loop<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
        &self,
        initial_game_state: Option<PartialGameState<MAX_BCOUNT, B_MAX_CAP>>
    ) -> Result<PartialGameState<MAX_BCOUNT, B_MAX_CAP>, UiRunError> {
        let mut state = SetupMenuState::new(initial_game_state);
        loop {
            let mut out = stdout();
            out.queue(Clear(ClearType::All))?.queue(MoveTo(0, 0))?;
            state.queue_display(&mut out)?;
            out.flush()?;
            let should_exit = state.handle_event(event::read()?)?;
            if should_exit {
                break Ok(state.gs);
            }
        }
    }

    /// Runs a loop that progressively demystifies a [PartialGameState] into a [KnownGameState]
    /// that can be solved properly.
    pub fn demystifier_loop<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
        &self,
        initial_gs: PartialGameState<MAX_BCOUNT, B_MAX_CAP>
    ) -> Result<DemystificationResult<MAX_BCOUNT, B_MAX_CAP>, UiRunError> {
        //this will be the gamestate the user actually interacts with
        let mut working_gs = initial_gs.clone();

        //this will be the gamestate that tracks what to reset to if we need to reset. we need it to be mutable
        //so we can update the unknown colors as they're revealed
        let mut initial_gs = initial_gs;

        loop {
            //first thing we do: if the working state can be converted into a known state, do the conversion and return.
            if let Ok(working_state_as_known) = KnownGameState::try_from(working_gs.clone()) {
                return Ok(DemystificationResult {
                    initial_state: initial_gs.try_into().expect("working state converted from partial to known, but initial state couldn't convert!"),
                current_state: working_state_as_known
            });
            }

            //try to find a solution to this PartialGameState - this will be a partial state with at least one unknown color on top
            let found_solution = self.solution_finding_loop(&working_gs)?;
            if let Some(found_solution) = found_solution {
                //TODO: specialize the message shown to say something other than "found solution"
                self.solution_viewer_loop(&found_solution)?;
                let pours = found_solution.take_pours();
                for pour in pours {
                    working_gs = pour
                        .try_apply(&working_gs)
                        .expect("invalid pour from solution");
                }

                //TODO: find some way to limit editing to only the revealed bottle
                //so that our assumption about the correspondance between known colors in working_gs and unknown colors initial_gs always holds true,
                //and to make entering the new info more convinient
                //for now, this only works if the user chooses only to update unknown colors
                working_gs = self.setup_menu_loop(Some(working_gs))?;

                //identify unknown colors in initial_gs that are known in working_gs, then update initial_gs accordingly
                for (bottle_idx, initial_bottle) in
                    initial_gs.get_mut_bottles().iter_mut().enumerate()
                {
                    let working_bottle = working_gs.get_bottles().get(bottle_idx)
                .expect("bottle in initial_gs had no corresponding bottle in working_gs; this happens when number of working_gs bottles is modified unexpectedly");

                    for color_idx in (0..initial_bottle.capacity()).rev() {
                        let initial_bottle_sample_result = initial_bottle.sample_at(color_idx);

                        if let BottleSampleResult::KnownColor(color) =
                            working_bottle.sample_at(color_idx)
                        {
                            if initial_bottle_sample_result == BottleSampleResult::UnknownColor {
                                initial_bottle
                                .try_set_color(
                                    color_idx,
                                    Some(PartialColoredWaterUnit::Color(color))
                                )
                                .expect(
                                    "Failed to update initial bottle with working bottle's content"
                                );
                            }
                        }
                    }
                }
            } else {
                // no solution to the partial state could be found - we can't get anywhere useful from here
                // therefore, we must reset to our initial state and try again
                // TODO: have some indication to the user that this is what's happening instead of just resetting
                // without a word
                working_gs = initial_gs.clone();
            }
        }
    }

    /// Runs a loop that handles display and input while the given [SolvableGameState] is solved
    /// in the background.
    ///
    /// Returns an [`Option<Solution>`]; if [None], no solution could be found.
    pub fn solution_finding_loop<'a, GamestateT: SolvableGameState>(
        &self,
        gamestate_to_solve: &'a GamestateT
    ) -> Result<Option<Solution<'a, GamestateT>>, UiRunError> {
        let mut state = WaitScreenState::new(gamestate_to_solve);
        let mut out = stdout();
        loop {
            let is_finished = state.check_finished();
            out.queue(Clear(ClearType::All))?.queue(MoveTo(0, 0))?;
            state.queue_display(&mut out)?;
            out.flush()?;

            // wait to handle an event if we're finished;
            // if we're not finished, handle events only if there are any events to be handled
            let should_exit = if is_finished || event::poll(Duration::from_millis(16))? {
                state.handle_event(event::read()?)?
            } else {
                false
            };

            if should_exit {
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
    pub fn solution_viewer_loop<GamestateT: SolvableGameState>(
        &self,
        solution: &Solution<GamestateT>
    ) -> Result<(), UiRunError> {
        let mut state = SolutionViewerState::new(solution);
        loop {
            let mut out = stdout();
            out.queue(Clear(ClearType::All))?.queue(MoveTo(0, 0))?;
            state.queue_display(&mut out)?;
            out.flush()?;
            let should_exit = state.handle_event(event::read()?)?;
            if should_exit {
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

/// Return value from successful demystification
pub struct DemystificationResult<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    /// The original [PartialGameState] that was demystified, with all unknown colors replaced
    /// with their now known values. If the game is reset, this will be the state reset to.
    pub initial_state: KnownGameState<MAX_BCOUNT, B_MAX_CAP>,

    /// The current state after the demystification process. This may or may not be solvable, as demystification
    /// shuffles colors around and can put the game into an unwinnable state.
    pub current_state: KnownGameState<MAX_BCOUNT, B_MAX_CAP>
}
