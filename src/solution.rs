//! Implementation of a [Solution]

use std::collections::{HashMap, VecDeque};

use crate::gamestate::{GameState, Pour};

/// A series of [Pour]s that, when applied to some [GameState]
/// in sequence, results in a [GameState] that is finished.
///
/// Although represented as [Pour]s, each [Pour] is guaranteed
/// to be valid for the result of the previous [Pour].
pub struct Solution<'a> {
    base_gamestate: &'a GameState,
    pours: VecDeque<Pour>
}

impl<'a> Solution<'a> {
    /// Try to find and return a Solution to the given GameState. If no solution can be found, return `None`.
    pub fn try_new(base_gamestate: &'a GameState) -> Option<Self> {
        let mut new_solution = Self {
            base_gamestate,
            pours: VecDeque::new()
        };

        if new_solution.add_pours(base_gamestate) {
            Some(new_solution)
        } else {
            None
        }
    }

    /// Given a GameState, generate possible pours and identify one that leads to a Solution.
    /// Once one is identified, add it to the *front* of the pours dequeue - because the first move
    /// that knows it is a winning move will be the last pour that should actually be applied.
    ///
    /// Returns true if this specific call added any pours false if not.
    fn add_pours(&mut self, gamestate_to_solve: &GameState) -> bool {
        //map from gamestate to (source_gamestate, pour_for_source_gamestate)
        //this allows us to track gamestates
        let mut tried_gamestates: HashMap<GameState, (GameState, Pour)> = HashMap::new();
        let mut gamestates_to_try: Vec<GameState> = vec![gamestate_to_solve.clone()];

        while let Some(gamestate_to_try) = gamestates_to_try.pop() {
            if gamestate_to_try.is_finished() {
                let mut gs = gamestate_to_try;
                loop {
                    if let Some((source_gs, pour)) = tried_gamestates.get(&gs) {
                        self.pours.push_front(pour.clone());
                        gs = source_gs.clone();
                    } else {
                        return true;
                    }
                }
            }

            for valid_pour in gamestate_to_try.iter_pours() {
                let new_gs = valid_pour.apply();
                // only check this gamestate if we haven't already checked it
                // this means it isn't in tried_gamestates (we didn't generate it)
                // *and* it isn't the gamestate we started with
                if !tried_gamestates.contains_key(&new_gs) && new_gs != *gamestate_to_solve {
                    tried_gamestates.insert(
                        new_gs.clone(),
                        (gamestate_to_try.clone(), valid_pour.into())
                    );
                    gamestates_to_try.push(new_gs);
                }
            }
        }

        // we iterated through all possible pours from this gamestate and found no pours that
        // lead to a solution; it's not possible to win from here, so return false
        false
    }

    pub fn get_base_gamestate(&self) -> &GameState {
        self.base_gamestate
    }

    pub fn get_pours(&self) -> &VecDeque<Pour> {
        &self.pours
    }
}
