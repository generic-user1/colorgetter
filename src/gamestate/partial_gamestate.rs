use super::*;
use crate::{
    bottle::{Bottle, PartialBottle, PartialBottleConversionError},
    colored_water::PartialColoredWaterUnit
};
use heapless::{CapacityError, Vec};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
mod pseudo_partial_gamestate;
pub use pseudo_partial_gamestate::{PseudoPartialGameState, PseudoPartialGameStateError};

/// The state a particular game is in, including [PartialBottle]s
/// instead of regular [KnownBottle](crate::bottle::KnownBottle)s
#[derive(Debug, Clone, Deserialize, Serialize, Eq)]
pub struct PartialGameState<
    const MAX_BCOUNT: usize,
    const B_MAX_CAP: usize,
    const ORDERLESS: bool = false
> {
    pub bottles: Vec<PartialBottle<B_MAX_CAP>, MAX_BCOUNT>
}

/// Given some [PartialGameState], return a [heapless::Vec] that contains references to its bottles.
///
/// If `ORDERLESS` is true for the state, the returned Vec will be reordered: all bottles containing only known units first
/// (and sorted into standard order), and all bottles containing at least some unknown units last (in their order of appearance).
///
/// If `ORDERLESS` is false for the state, the returned Vec will *not* be reordered.
///
/// The intent is for this function to be responsible for all reordering of bottles as part of orderless Eq, Ord, and Hash.
/// This means that when `ORDERLESS == true`, the implementations are orderless, and when `ORDERLESS == false`, the implementations aren't orderless.
fn conditional_sort_bottles<
    const MAX_BCOUNT: usize,
    const B_MAX_CAP: usize,
    const ORDERLESS: bool
>(
    given_state: &PartialGameState<MAX_BCOUNT, B_MAX_CAP, ORDERLESS>
) -> heapless::Vec<&PartialBottle<B_MAX_CAP>, MAX_BCOUNT> {
    //which path to take depends on the value of ORDERLESS
    if ORDERLESS {
        //ORDERLESS == true, do the reordering
        let mut output = heapless::Vec::new();

        //first, place all known-only bottles into the output
        for bottle in given_state
            .bottles
            .iter()
            .filter(|b| b.get_unknown_count() == 0)
        {
            output
                .push(bottle)
                .expect("ran out of capacity while inserting known-only bottles");
        }
        //sort the known-only bottles
        output.sort();
        //insert the remaining bottles (those that contain unknown units)
        for bottle in given_state
            .bottles
            .iter()
            .filter(|b| b.get_unknown_count() > 0)
        {
            output
                .push(bottle)
                .expect("ran out of capacity while inserting unknown-containing bottles")
        }
        output
    } else {
        //ORDERLESS == false; we don't need reordering and just create a Vec of references
        given_state.bottles.iter().collect()
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize, const ORDERLESS: bool> Ord
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP, ORDERLESS>
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let our_std_order_bottles = conditional_sort_bottles(self);
        let other_std_order_bottles = conditional_sort_bottles(other);
        our_std_order_bottles.cmp(&other_std_order_bottles)
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize, const ORDERLESS: bool> PartialOrd
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP, ORDERLESS>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize, const ORDERLESS: bool> PartialEq
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP, ORDERLESS>
{
    fn eq(&self, other: &Self) -> bool {
        let our_std_order_bottles = conditional_sort_bottles(self);
        let other_std_order_bottles = conditional_sort_bottles(other);

        our_std_order_bottles == other_std_order_bottles
    }
}
impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize, const ORDERLESS: bool> Hash
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP, ORDERLESS>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let std_order_bottles = conditional_sort_bottles(self);

        for bottle in std_order_bottles {
            bottle.hash(state);
        }
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> TryFrom<&[PartialBottle<B_MAX_CAP>]>
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP>
{
    type Error = CapacityError;
    /// This will only fail if the number of [PartialBottle]s in the provided `value` exceeds the desired `B_MAX_CAP`.
    fn try_from(value: &[PartialBottle<B_MAX_CAP>]) -> Result<Self, Self::Error> {
        Ok(PartialGameState {
            bottles: Vec::from_slice(value)?
        })
    }
}
impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> From<[PartialBottle<B_MAX_CAP>; MAX_BCOUNT]>
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP>
{
    fn from(value: [PartialBottle<B_MAX_CAP>; MAX_BCOUNT]) -> Self {
        PartialGameState {
            bottles: Vec::from_slice(&value).unwrap()
        }
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>
    TryFrom<PartialGameState<MAX_BCOUNT, B_MAX_CAP>> for KnownGameState<MAX_BCOUNT, B_MAX_CAP>
{
    type Error = PartialBottleConversionError;
    /// Direct conversion from [PartialGameState] into [KnownGameState] is allowed as long as all [PartialBottle]s
    /// within the [PartialGameState] can be converted into [KnownBottle](crate::bottle::KnownBottle)s.
    fn try_from(value: PartialGameState<MAX_BCOUNT, B_MAX_CAP>) -> Result<Self, Self::Error> {
        let mut converted_bottles = Vec::new();
        for bottle in value.bottles {
            converted_bottles.push(bottle.try_into()?).unwrap();
        }

        Ok(KnownGameState {
            bottles: converted_bottles
        })
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize, const ORDERLESS: bool> GameState
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP, ORDERLESS>
{
    type BottleT = PartialBottle<B_MAX_CAP>;

    fn get_bottles(&self) -> &[Self::BottleT] {
        &self.bottles
    }

    fn get_mut_bottles(&mut self) -> &mut [Self::BottleT] {
        &mut self.bottles
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize, const ORDERLESS: bool> SolvableGameState
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP, ORDERLESS>
{
    /// Returns whether this GameState is solved
    ///
    /// For [PartialGameState], "solved" means that at least one bottle has an unknown color at the top,
    /// or that there are no unknown colors at all (i.e. the [PartialGameState] can be converted into a [KnownGameState])
    fn is_solved(&self) -> bool {
        let mut found_any_unknown = false;
        for bottle in &self.bottles {
            match bottle.get_top_color() {
                Some(PartialColoredWaterUnit::Color(_)) => (),
                Some(PartialColoredWaterUnit::UnknownColor) => return true,
                None => ()
            }
            if bottle.get_unknown_count() != 0 {
                found_any_unknown = true;
            }
        }
        !found_any_unknown
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>
    From<PartialGameState<MAX_BCOUNT, B_MAX_CAP, true>>
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP, false>
{
    fn from(value: PartialGameState<MAX_BCOUNT, B_MAX_CAP, true>) -> Self {
        PartialGameState {
            bottles: value.bottles
        }
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>
    From<PartialGameState<MAX_BCOUNT, B_MAX_CAP, false>>
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP, true>
{
    fn from(value: PartialGameState<MAX_BCOUNT, B_MAX_CAP, false>) -> Self {
        PartialGameState {
            bottles: value.bottles
        }
    }
}
