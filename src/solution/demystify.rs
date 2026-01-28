//! utilities for demystification of [PartialGameState]s into [KnownGameState](crate::gamestate::KnownGameState)s

use super::*;
use crate::gamestate::PartialGameState;

/// Statistics regarding confidence in [try_demystify_next_step]'s result
#[derive(Default)]
pub struct DemystifyNextStepStats {
    /// The number of possible [KnownGameState](crate::gamestate::KnownGameState)s that
    /// were checked for a solution.
    pub possible_states_checked: usize,

    /// The number of possible [KnownGameState](crate::gamestate::KnownGameState)s that
    /// were solved in order to find the next step. This will always be less than or equal to `possible_states_checked`.
    pub solutions_found: usize,

    /// The number of possible [KnownGameState](crate::gamestate::KnownGameState)s whose
    /// solutions start with the provided `next_step`. This will always be less than or equal to `solutions_found`.
    /// The higher this number is, the more confidence there can be in the `next_step` being of good quality.
    pub solutions_sharing_prefix: usize
}

/// Try to find a [Solution] for the given [PartialGameState] that leads to revealing a new unknown color unit
/// while using a prediction technique to try and prevent dead-ends. Similar to [Solution::try_new], will return [None]
/// if no [Solution] can be found.
///
/// This returns the found [Solution] along with an instance of [DemystifyNextStepStats] so the level of confidence
/// in the answer can be communicated
pub fn try_demystify_next_step<'a, const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gamestate_to_solve: &'a PartialGameState<MAX_BCOUNT, B_MAX_CAP>
) -> Option<(
    Solution<'a, PartialGameState<MAX_BCOUNT, B_MAX_CAP>>,
    DemystifyNextStepStats
)> {
    Solution::try_new(gamestate_to_solve, 0).map(|s| (s, DemystifyNextStepStats::default()))
}
