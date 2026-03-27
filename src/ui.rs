//! Implementation of a user interface for setting up [KnownGameState]s/[PartialGameState]s

use std::{
    io::{self, stdout, Write},
    iter,
    marker::PhantomData,
    sync::Mutex,
    time::Duration
};

use core::ops::Drop;

use crossterm::{
    cursor::{Hide, MoveDown, MoveTo, MoveToColumn, Show},
    event,
    style::{Attributes, Color, ContentStyle, Print},
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
    solution::{try_demystify_next_step, Solution},
    ui::setup_menu::save_menu_loop
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
use solution_wait_screen::WaitScreenState;

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
        self.setup_menu_loop_inner(initial_game_state, None)
    }

    /// Inner logic of [Ui::setup_menu_loop]; offers the same functionality with the added ability
    /// to restrict the cursor to a particular bottle for demystification purposes.
    ///
    /// If `specific_bottle_idx` is specified and valid, the setup menu will:
    ///     
    /// - default the cursor position to the topmost unit within that bottle
    ///     
    /// - only allow units within that bottle to be edited; specifically, only units
    ///   that were unknown colors when this function was called
    ///
    /// - entirely disable editing of the number of bottles or bottle capacity and hide messages associated with
    ///   those features
    ///
    /// If `specific_bottle_idx` is unspecified or invalid, all setup menu functionality is enabled.
    /// `specific_bottle_idx` is considered invalid when:
    ///     
    /// - `initial_game_state` is `None`
    ///
    /// - `initial_game_state` has no bottle at the given index
    ///
    /// - the bottle specified by `initial_game_state` has zero unknown units
    fn setup_menu_loop_inner<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
        &self,
        initial_game_state: Option<PartialGameState<MAX_BCOUNT, B_MAX_CAP>>,
        specific_bottle_idx: Option<usize>
    ) -> Result<PartialGameState<MAX_BCOUNT, B_MAX_CAP>, UiRunError> {
        let mut state = SetupMenuState::new(initial_game_state, specific_bottle_idx);
        let mut out = stdout();
        loop {
            state.clear_screen_if_needed(&mut out)?;
            out.queue(MoveTo(0, 0))?;
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
            if let Some(found_solution) = self.next_color_wait_loop(&working_gs)? {
                self.solution_viewer_loop(&found_solution)?;
                let pours = found_solution.take_pours();

                // this var will track the final bottle poured from so we can limit
                // the unknown-to-known setup menu to that bottle, as it's the only one that
                // should now have an unknown unit on top.
                let mut last_source_idx = None;

                for pour in pours {
                    last_source_idx = Some(pour.source_bottle_index);
                    working_gs = pour
                        .try_apply(&working_gs)
                        .expect("invalid pour from solution");
                }
                // if last_source_idx was never set, (probably because the gamestate was solved
                // from the get-go and the solution was a no-op), we'll try to use the first bottle
                // with an unknown unit on top as a fallback
                if last_source_idx.is_none() {
                    for (idx, bottle) in working_gs.bottles.iter().enumerate() {
                        if bottle.get_top_color() == Some(PartialColoredWaterUnit::UnknownColor) {
                            last_source_idx = Some(idx);
                            break;
                        }
                    }
                }

                working_gs = self.setup_menu_loop_inner(Some(working_gs), last_source_idx)?;

                // identify unknown colors in initial_gs's bottle at last_source_idx
                // whose equivalents in working_gs are known
                if let Some(last_source_idx) = last_source_idx {
                    let initial_bottle = initial_gs
                        .get_mut_bottles()
                        .get_mut(last_source_idx)
                        .expect("bottle in initial_gs at last_source_idx doesn't exist");
                    let working_bottle = working_gs
                        .get_bottles()
                        .get(last_source_idx)
                        .expect("bottle in working_gs at last_source_idx doesn't exist");

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
                working_gs = initial_gs.clone();
            }
        }
    }

    /// Handles the details of finding the path to the next color during demystification
    ///
    /// Split out from demystifier_loop due to complexity, not due to general usefulness
    fn next_color_wait_loop<'a, const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
        &self,
        gs: &'a PartialGameState<MAX_BCOUNT, B_MAX_CAP>
    ) -> Result<Option<Solution<'a, PartialGameState<MAX_BCOUNT, B_MAX_CAP>>>, UiRunError> {
        //if our given gamestate is already solved, exit early and skip the expensive try_demystify_next_step call
        if gs.is_solved() {
            return Ok(Some(Solution::try_from_parts(gs, iter::empty()).unwrap()));
        }

        let inner_gs = gs.clone();
        let mut state = WaitScreenState::new(move || {
            try_demystify_next_step(&inner_gs)
                .map(|(solution, stats)| (solution.take_pours(), stats))
        });

        let mut out = stdout();
        out.queue(Clear(ClearType::All))?;
        loop {
            let is_finished = state.check_finished();
            out.queue(MoveTo(0, 0))?;
            if is_finished {
                out.queue(Clear(ClearType::All))?.queue(Print(
                    if let Some((_, stats)) = state.borrow_result().unwrap() {
                        format!("Found path to next color. {} possible solution{} checked, {} was the min score found ({} solution{} with this score)", 
                        stats.solutions_checked, if stats.solutions_checked == 1 {""} else {"s"},
                        stats.min_score, 
                        stats.equal_scoring_solution_count, if stats.equal_scoring_solution_count == 1 {""} else {"s"}
                    )
                    } else {
                        "No path to next color found; reset game before continuing".to_owned()
                    }
                ))?;
            } else {
                out.queue(Print("Finding path to next color..."))?;
            }

            out.queue(MoveDown(1))?.queue(MoveToColumn(0))?;
            state.queue_display(&mut out)?;
            out.flush()?;

            // wait to handle an event if we're finished;
            // if we're not finished, handle events only if there are any events to be handled
            if is_finished || event::poll(Duration::from_millis(16))? {
                let should_exit = state.handle_event(event::read()?)?;

                if should_exit {
                    let solution = state.take_result().map(|(pours, _)| {
                        Solution::try_from_parts(gs, pours).expect("solution wasn't valid")
                    });
                    return Ok(solution);
                }
            }
        }
    }

    /// Runs a loop that handles display and input while the given [DemystificationResult] is solved
    /// in the background.
    ///
    /// This will first try to solve the [DemystificationResult::current_state]. If no solution can be found,
    /// will inform the user that a reset is required, then try to solve the [DemystificationResult::initial_state].
    ///
    /// Returns an [`Option<Solution>`]; if [None], no solution could be found for either state.
    pub fn demystified_result_solution_finding_loop<
        'a: 'b,
        'b,
        const MAX_BCOUNT: usize,
        const B_MAX_CAP: usize
    >(
        &self,
        result_to_solve: &'a DemystificationResult<MAX_BCOUNT, B_MAX_CAP>
    ) -> Result<Option<Solution<'b, KnownGameState<MAX_BCOUNT, B_MAX_CAP>>>, UiRunError> {
        let current_state_solution =
            self.solution_finding_loop_inner(&result_to_solve.current_state, true)?;
        if current_state_solution.is_some() {
            return Ok(current_state_solution);
        } else if result_to_solve.current_state != result_to_solve.initial_state {
            let initial_state_solution =
                self.solution_finding_loop_inner(&result_to_solve.initial_state, false)?;
            return Ok(initial_state_solution);
        }
        Ok(None)
    }

    /// Runs a loop that displays a dialog for saving a demystified state
    ///
    /// Will save the [DemystificationResult::initial_state] to whatever file path the user chooses,
    /// unless the user chooses to cancel (in which case, nothing is saved).
    ///
    /// Returns the file path saved to, or None if the user chose to cancel.
    pub fn save_demystified<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
        &self,
        result_to_save: &DemystificationResult<MAX_BCOUNT, B_MAX_CAP>
    ) -> Result<Option<String>, UiRunError> {
        save_menu_loop(&result_to_save.initial_state)
    }

    /// Runs a loop that handles display and input while the given [KnownGameState] is solved
    /// in the background.
    ///
    /// Note that this only handles [KnownGameState]s. If you instead have a [PartialGameState], you likely want to use
    /// [Ui::demystifier_loop] and [Ui::demystified_result_solution_finding_loop] instead of this function.
    ///
    /// Returns an [`Option<Solution>`]; if [None], no solution could be found.
    pub fn solution_finding_loop<'a, const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
        &self,
        gamestate_to_solve: &'a KnownGameState<MAX_BCOUNT, B_MAX_CAP>
    ) -> Result<Option<Solution<'a, KnownGameState<MAX_BCOUNT, B_MAX_CAP>>>, UiRunError> {
        self.solution_finding_loop_inner(gamestate_to_solve, false)
    }

    /// Inner logic for [Ui::solution_finding_loop]; allows controlling whether to prompt for game reset
    fn solution_finding_loop_inner<'a, GamestateT: SolvableGameState>(
        &self,
        gamestate_to_solve: &'a GamestateT,
        prompt_for_reset: bool
    ) -> Result<Option<Solution<'a, GamestateT>>, UiRunError> {
        let inner_gamestate_to_solve = gamestate_to_solve.clone();
        let mut state = WaitScreenState::new(move || {
            Solution::try_new(&inner_gamestate_to_solve, 0).map(|x| x.take_pours())
        });
        let mut out = stdout();
        out.queue(Clear(ClearType::All))?;
        loop {
            let is_finished = state.check_finished();
            out.queue(MoveTo(0, 0))?;
            if is_finished {
                out.queue(Clear(ClearType::All))?.queue(Print(
                    if state.borrow_result().unwrap().is_some() {
                        "Found solution"
                    } else if prompt_for_reset {
                        "No solution found; reset game before continuing"
                    } else {
                        "No solution found"
                    }
                ))?;
            } else {
                out.queue(Print("Finding solution..."))?;
            }
            out.queue(MoveDown(1))?.queue(MoveToColumn(0))?;
            state.queue_display(&mut out)?;
            out.flush()?;

            // wait to handle an event if we're finished;
            // if we're not finished, handle events only if there are any events to be handled
            if is_finished || event::poll(Duration::from_millis(16))? {
                let should_exit = state.handle_event(event::read()?)?;

                if should_exit {
                    let solution = state.take_result().map(|pours| {
                        Solution::try_from_parts(gamestate_to_solve, pours)
                            .expect("solution wasn't valid")
                    });
                    return Ok(solution);
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
        let mut out = stdout();
        out.queue(Clear(ClearType::All))?;
        loop {
            out.queue(MoveTo(0, 0))?;
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
