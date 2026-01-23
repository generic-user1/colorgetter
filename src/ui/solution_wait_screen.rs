use super::UiRunError;
use crossterm::{
    cursor::{MoveDown, MoveToColumn},
    event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::Print,
    QueueableCommand
};
use std::{
    io,
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
    time::{Duration, Instant}
};

/// A struct for running some arbitrary job in a different thread and handling user input
/// and printing runtime + job status while waiting for that job to finish.
pub(super) struct WaitScreenState<T: Send + 'static> {
    search_start_time: Instant,
    waiting_thread_handle: Option<JoinHandle<T>>,
    thread_result: Option<T>,
    search_end_time: Arc<RwLock<Option<Instant>>>
}

impl<T: Send + 'static> WaitScreenState<T> {
    pub fn new<U>(job_to_wait_on: U) -> WaitScreenState<T>
    where
        U: FnOnce() -> T + Send + 'static
    {
        let search_start_time = Instant::now();
        let search_end_time = Arc::new(RwLock::new(None));
        let search_end_time_inner = search_end_time.clone();
        let handle = thread::spawn(move || {
            let result = job_to_wait_on();
            let end_time = Instant::now();
            *search_end_time_inner.write().expect("main thread panicked") = Some(end_time);
            result
        });
        WaitScreenState {
            search_start_time,
            waiting_thread_handle: Some(handle),
            thread_result: None,
            search_end_time
        }
    }

    /// Determine whether the job being waited on has finished (`true`) or is still in progress (`false`).
    ///
    /// Will update internal state to set finish time and store job result if this call determines that the search is newly finished.
    pub fn check_finished(&mut self) -> bool {
        if self.waiting_thread_handle.is_some() {
            // if our thread handle still exists, the thread is either still running,
            // or has completed and we just haven't processed the result.
            if self.waiting_thread_handle.as_ref().unwrap().is_finished() {
                // take the thread handle out of the option, replacing the `self.waiting_thread_handle` with None
                let handle = self.waiting_thread_handle.take().unwrap();
                // pull the result out of the handle, put it into self.thread_result
                self.thread_result = Some(handle.join().expect("waiting thread panicked"));
                true
            } else {
                false
            }
        } else {
            true
        }
    }

    /// Borrow the result of the job being waited on in the Wait Screen if possible
    ///
    /// If the job has not yet finished, returns None.
    /// If the job has finished but has no result, panics, as this should never happen.
    ///
    /// Will update internal state to set finish time and store job result if this call determines that the search is newly finished.
    pub fn borrow_result(&mut self) -> Option<&T> {
        if self.check_finished() {
            if self.thread_result.is_some() {
                self.thread_result.as_ref()
            } else {
                panic!("job finished, but result was missing")
            }
        } else {
            None
        }
    }

    /// Take ownership of the result of the job being waited on in the Wait Screen, waiting
    /// on the job to finish if necessary.
    pub fn take_result(self) -> T {
        if let Some(handle) = self.waiting_thread_handle {
            handle.join().expect("waiting thread panicked")
        } else {
            self.thread_result
                .expect("job finished, but result was missing")
        }
    }

    /// Get a [Duration] representing the amount of time spent waiting.
    /// This will be the time since this WaitScreenState's creation if the job is still in progress,
    /// but will be set-in-stone once the job has completed.
    ///
    /// Will update internal state to set finish time and store job result if this call determines that the job is newly finished.
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

    /// Display runtime, whether the job is still running, and a button prompt.
    /// Does not describe what specifically the job is; that must be done by the caller.
    pub fn queue_display<U: QueueableCommand>(&mut self, ostream: &mut U) -> io::Result<()> {
        let is_finished = self.check_finished();
        let runtime = self.get_runtime();
        if is_finished {
            ostream.queue(Print(format!("Finished after running for {:?}", runtime)))?;
        } else {
            ostream.queue(Print(format!("Running for {:?}", runtime)))?;
        }
        ostream.queue(MoveDown(1))?.queue(MoveToColumn(0))?;
        if is_finished {
            ostream.queue(Print("Press any key to continue"))?;
        } else {
            ostream.queue(Print("Press CTRL+C to abort"))?;
        }
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
                KeyEvent { kind: k, .. }
                    if k == KeyEventKind::Press || k == KeyEventKind::Repeat =>
                {
                    if self.check_finished() {
                        return Ok(true);
                    }
                }
                _ => ()
            }
        }
        Ok(false)
    }
}
