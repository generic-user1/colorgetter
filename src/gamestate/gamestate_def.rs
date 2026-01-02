use super::*;
use crate::bottle::KnownBottle;
use heapless::{CapacityError, Vec};
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// The state a particular game is in, including only [KnownBottle]s
///
/// That is, represents what bottles exist and what order they're in.
#[derive(Debug, Clone, Eq, Deserialize, Serialize)]
pub struct KnownGameState<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    pub bottles: Vec<KnownBottle<B_MAX_CAP>, MAX_BCOUNT>
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> KnownGameState<MAX_BCOUNT, B_MAX_CAP> {
    /// Returns whether this KnownGameState represents a finished game
    ///
    /// The game is finished when all bottles are either completely empty or
    /// completely full of a single color
    pub fn is_finished(&self) -> bool {
        for bottle in &self.bottles {
            if !(bottle.is_in_final_state() || bottle.get_content().is_empty()) {
                return false;
            }
        }
        true
    }

    /// Returns an iterator over all [ValidPour](crate::gamestate::ValidPour)s you could apply to this KnownGameState
    pub fn iter_pours(&self) -> ValidPourIter<'_, MAX_BCOUNT, B_MAX_CAP> {
        ValidPourIter::new(self)
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> Ord
    for KnownGameState<MAX_BCOUNT, B_MAX_CAP>
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let mut our_std_order_bottles = self.bottles.clone();
        our_std_order_bottles.sort();

        let mut other_std_order_bottles = other.bottles.clone();
        other_std_order_bottles.sort();

        our_std_order_bottles.cmp(&other_std_order_bottles)
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> PartialOrd
    for KnownGameState<MAX_BCOUNT, B_MAX_CAP>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> PartialEq
    for KnownGameState<MAX_BCOUNT, B_MAX_CAP>
{
    fn eq(&self, other: &Self) -> bool {
        if self.is_finished() != other.is_finished() {
            return false;
        }

        let mut our_std_order_bottles = self.bottles.clone();
        our_std_order_bottles.sort();

        let mut other_std_order_bottles = other.bottles.clone();
        other_std_order_bottles.sort();

        our_std_order_bottles == other_std_order_bottles
    }
}
impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> Hash
    for KnownGameState<MAX_BCOUNT, B_MAX_CAP>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Include whether we're finished
        self.is_finished().hash(state);

        // Hash for this KnownGameState is the hashes of all bottles in the gamestate in some standard order
        // To accomplish this, we first put the bottles of this state into standard order
        let mut std_order_bottles = self.bottles.clone();
        std_order_bottles.sort();

        // we then hash bottles in said order
        for bottle in std_order_bottles {
            bottle.hash(state);
        }
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> TryFrom<&[KnownBottle<B_MAX_CAP>]>
    for KnownGameState<MAX_BCOUNT, B_MAX_CAP>
{
    type Error = CapacityError;
    /// This will only fail if the number of [KnownBottle]s in the provided `value` exceeds the desired `B_MAX_CAP`.
    fn try_from(value: &[KnownBottle<B_MAX_CAP>]) -> Result<Self, Self::Error> {
        Ok(KnownGameState {
            bottles: Vec::from_slice(value)?
        })
    }
}
impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> From<[KnownBottle<B_MAX_CAP>; MAX_BCOUNT]>
    for KnownGameState<MAX_BCOUNT, B_MAX_CAP>
{
    fn from(value: [KnownBottle<B_MAX_CAP>; MAX_BCOUNT]) -> Self {
        KnownGameState {
            bottles: Vec::from_slice(&value).unwrap()
        }
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> From<KnownGameState<MAX_BCOUNT, B_MAX_CAP>>
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP>
{
    fn from(value: KnownGameState<MAX_BCOUNT, B_MAX_CAP>) -> Self {
        let mut converted_bottles = Vec::new();
        for bottle in value.bottles {
            converted_bottles.push(bottle.into()).unwrap();
        }

        PartialGameState {
            bottles: converted_bottles
        }
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> GameStateDisplay
    for KnownGameState<MAX_BCOUNT, B_MAX_CAP>
{
    type BottleT = KnownBottle<B_MAX_CAP>;

    fn get_bottles(&self) -> &[Self::BottleT] {
        &self.bottles
    }
}
