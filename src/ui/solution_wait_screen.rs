use crate::{
    gamestate::{GameState, Pour},
    solution::Solution
};

use super::UiRunError;
use crossterm::{
    cursor::{MoveDown, MoveToColumn},
    event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::Print,
    QueueableCommand
};
use std::{
    collections::VecDeque,
    io,
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
    time::{Duration, Instant}
};

pub(super) struct WaitScreenState<'a> {
    search_start_time: Instant,
    gamestate_to_solve: &'a GameState,
    solver_thread_handle: Option<JoinHandle<Option<VecDeque<Pour>>>>,
    pours: Option<VecDeque<Pour>>,
    search_end_time: Arc<RwLock<Option<Instant>>>,
    pub should_exit: bool
}

impl<'a> WaitScreenState<'a> {
    pub fn new(gamestate_to_solve: &'a GameState) -> WaitScreenState<'a> {
        let gs = gamestate_to_solve.clone();
        let search_start_time = Instant::now();
        let search_end_time = Arc::new(RwLock::new(None));
        let search_end_time_inner = search_end_time.clone();
        let handle = thread::spawn(move || {
            let pours = Solution::try_new(&gs, 0).map(|x| x.take_pours());
            let end_time = Instant::now();
            *search_end_time_inner.write().expect("main thread panicked") = Some(end_time);
            pours
        });
        WaitScreenState {
            search_start_time,
            gamestate_to_solve,
            solver_thread_handle: Some(handle),
            pours: None,
            search_end_time,
            should_exit: false
        }
    }

    /// Determine whether searching for a [Solution] has finished (`true`) or is still in progress (`false`).
    ///
    /// Will update internal state to set finish time and search result if this call determines that the search is newly finished.
    pub fn check_finished(&mut self) -> bool {
        if self.solver_thread_handle.is_some() {
            // if our thread handle still exists, the thread is either still running,
            // or has completed and we just haven't processed the result.
            if self.solver_thread_handle.as_ref().unwrap().is_finished() {
                // take the thread handle out of the option, replacing the `self.solver_thread_handle` with None
                let handle = self.solver_thread_handle.take().unwrap();
                // pull the Option<VecDeque<Pour>> out of the handle, put it into self.pours
                self.pours = handle.join().expect("solver thread panicked");
                true
            } else {
                false
            }
        } else {
            true
        }
    }

    /// Get the [Solution] found while waiting in the Wait Screen, if it exists.
    ///
    /// Will update internal state to set finish time and search result if this call determines that the search is newly finished.
    pub fn get_solution(&mut self) -> Result<Solution<'a>, GetSolutionError> {
        if !self.check_finished() {
            return Err(GetSolutionError::NotYetFinished);
        }

        // if we reach this point, we know that we're finished.
        if let Some(pours) = &self.pours {
            Ok(
                Solution::try_from_parts(self.gamestate_to_solve, pours.clone())
                    .expect("solution was not valid")
            )
        } else {
            Err(GetSolutionError::NoSolutionFound)
        }
    }

    /// Get a [Duration] representing the amount of time spent searching for a [Solution].
    /// This will be the time since this WaitScreenState's creation if solution searching is still in progress,
    /// but will be set-in-stone once the searching has completed.
    ///
    /// Will update internal state to set finish time and search result if this call determines that the search is newly finished.
    pub fn get_runtime(&mut self) -> Duration {
        if !self.check_finished() {
            Instant::now().duration_since(self.search_start_time)
        } else {
            self.search_end_time
                .read()
                .expect("solver thread panicked")
                .expect("finished without end time")
                .duration_since(self.search_start_time)
        }
    }

    pub fn queue_display<T: QueueableCommand>(&mut self, ostream: &mut T) -> io::Result<()> {
        let is_finished = self.check_finished();
        let runtime = self.get_runtime();
        if is_finished {
            let solution_found = self.pours.is_some();

            ostream.queue(Print(if solution_found {
                format!("Found solution in {:?}", runtime)
            } else {
                format!("Finished searching with no solution in {:?}", runtime)
            }))?;
        } else {
            ostream.queue(Print(format!("Searching for {:?}", runtime)))?;
        }
        ostream.queue(MoveDown(1))?.queue(MoveToColumn(0))?;
        if is_finished {
            ostream.queue(Print("Press any key to continue"))?;
        } else {
            ostream.queue(Print("Press CTRL+C to abort"))?;
        }
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
                KeyEvent { kind: k, .. }
                    if k == KeyEventKind::Press || k == KeyEventKind::Repeat =>
                {
                    if self.check_finished() {
                        self.should_exit = true;
                    }
                }
                _ => ()
            }
        }
        Ok(())
    }
}

/// Reasons why [WaitScreenState::get_solution] may fail
#[derive(Debug)]
pub enum GetSolutionError {
    /// No solution has been found yet, but processing is still in progress
    NotYetFinished,
    /// Processing has finished and no solution was found
    NoSolutionFound
}
