use crossterm::{event::Event, QueueableCommand};
use std::io;

use super::UiRunError;
use crate::solution::Solution;

pub(super) struct SolutionViewerState<'a, const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    pub solution: Solution<'a, MAX_BCOUNT, B_MAX_CAP>,
    pub should_exit: bool
}

impl<'a, const MAX_BCOUNT: usize, const B_MAX_CAP: usize>
    SolutionViewerState<'a, MAX_BCOUNT, B_MAX_CAP>
{
    pub fn new(
        solution: Solution<'a, MAX_BCOUNT, B_MAX_CAP>
    ) -> SolutionViewerState<'a, MAX_BCOUNT, B_MAX_CAP> {
        SolutionViewerState {
            solution,
            should_exit: false
        }
    }

    pub fn queue_display<T: QueueableCommand>(&self, ostream: &mut T) -> Result<(), UiRunError> {
        todo!()
    }

    pub fn handle_event(&mut self, event: Event) -> io::Result<()> {
        todo!()
    }
}
