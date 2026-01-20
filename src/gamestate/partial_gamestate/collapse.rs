//! Logic for "collapsing" (as in "wave function collapse") one PartialGameState
//! into either all or a randome sample of possible KnownGameStates it could be

use std::collections::{hash_map::Entry, HashMap};

use crate::{
    bottle::{Bottle, BottleSampleResult},
    colored_water::{ColoredWaterIter, ColoredWaterUnit, PartialColoredWaterUnit},
    gamestate::{KnownGameState, PartialGameState}
};

use itertools::Itertools;
use rand::{rngs::ThreadRng, seq::SliceRandom};

struct BasicShuffleIter {
    permutable: Vec<ColoredWaterUnit>,
    rng: ThreadRng
}
impl BasicShuffleIter {
    pub fn new(permutable: Vec<ColoredWaterUnit>) -> Self {
        Self {
            permutable,
            rng: rand::rng()
        }
    }
}
impl Iterator for BasicShuffleIter {
    type Item = Vec<ColoredWaterUnit>;
    fn next(&mut self) -> Option<Self::Item> {
        self.permutable.shuffle(&mut self.rng);
        Some(self.permutable.clone())
    }
}

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> PartialGameState<MAX_BCOUNT, B_MAX_CAP> {
    /// Tries to create and return an iterator over all possible [KnownGameState]s from this [PartialGameState]
    ///
    /// All possible [KnownGameState]s will eventually be yielded, but in a very specific order.
    /// This makes the iterator returned by this function very bad for collecting a representative sample
    /// of possibilities. You likely want to collect a representive sample instead of *all* possibilities
    /// when you have several unknown color units, since the number of possibilities scales (roughly) factorially
    /// with respect to the number of unknown units.
    ///
    /// If you have several unknown color units and wish to collect a representative sample, use [PartialGameState::collapse_random_sample].
    ///
    /// This function will fail to produce a result if this [PartialGameState] has bottles of varying capacity. This limitation
    /// may or may not be resolved in the future.
    pub fn collapse_all(
        &self
    ) -> Option<
        impl Iterator<Item = KnownGameState<MAX_BCOUNT, B_MAX_CAP>> + use<'_, MAX_BCOUNT, B_MAX_CAP>
    > {
        if let Some(permutable) = to_permutable(self) {
            let length = permutable.len();
            Some(
                permutable
                    .into_iter()
                    .permutations(length)
                    .unique()
                    .map(|permutation| permutation_as_known(self.clone(), permutation.into_iter()))
            )
        } else {
            None
        }
    }

    /// Tries to return an iterator over a random sample of [KnownGameState]s from this [PartialGameState]
    ///
    /// This iterator will never end, and will continue generating random possible [KnownGameState]s
    /// for as long as you want it to. It's useful for collecting a representative sample of possible states
    /// when the number of total possible states is too large to consider. However, it may produce duplicates,
    /// and it has no guarantee that all possible [KnownGameState]s will eventually be generated.
    ///
    /// If you are confident that the number of possible [KnownGameState]s is low (because there are very few unknown
    /// color units), consider using [PartialGameState::collapse_all] instead, as it's guaranteed to generate all possible states eventually.
    ///
    /// This function will fail to produce a result if this [PartialGameState] has bottles of varying capacity. This limitation
    /// may or may not be resolved in the future.
    pub fn collapse_random_sample(
        &self
    ) -> Option<
        impl Iterator<Item = KnownGameState<MAX_BCOUNT, B_MAX_CAP>> + use<'_, MAX_BCOUNT, B_MAX_CAP>
    > {
        to_permutable(self).map(|permutable| {
            BasicShuffleIter::new(permutable)
                .map(|permutation| permutation_as_known(self.clone(), permutation.into_iter()))
        })
    }
}

/// Generates a [Vec] of [ColoredWaterUnit]s from a [PartialGameState]
///
/// This vec contains one ColoredWaterUnit for each unknown color in the [PartialGameState]
/// It will have as many of each color as are "missing" from the [PartialGameState]. The vec
/// can then be shuffled around to generate valid possible KnownGameStates from the [PartialGameState].
///
/// This function will return None if the [PartialGameState] has bottles of varying capacity, as it cannot be determined
/// conclusively which color has how many units.
/// TODO: fix this limitation if it's possible, or determine conclusively that it's not possible
fn to_permutable<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gs: &PartialGameState<MAX_BCOUNT, B_MAX_CAP>
) -> Option<Vec<ColoredWaterUnit>> {
    //try to get the capacity of the first bottle. if there is no first bottle, immediately return an empty vec
    let first_bottle_cap = gs.bottles.first().map(|b| b.capacity());
    if first_bottle_cap.is_none() {
        return Some(Vec::new());
    }

    //this capacity will be the number of units of each color we expect, assuming that all bottles have equal capacity
    let count_per_color = first_bottle_cap.unwrap();
    //ensure the capacity of all bottles matches. if any bottle doesn't match, return None
    for bottle in &gs.bottles {
        if bottle.capacity() != count_per_color {
            return None;
        }
    }

    //build a mapping of known colors to their number of occurrances in the gamestate,
    //and count number of unknown color units
    let mut known_color_counts = HashMap::new();
    let mut total_unknown_units = 0_usize;
    for bottle in &gs.bottles {
        for idx in 0..bottle.capacity() {
            if let Some(color) = bottle.sample_content_at(idx) {
                match color {
                    PartialColoredWaterUnit::UnknownColor => total_unknown_units += 1,
                    PartialColoredWaterUnit::Color(c) => {
                        if let Some(count) = known_color_counts.get_mut(&c) {
                            *count += 1;
                        } else {
                            known_color_counts.insert(c, 1_usize);
                        }
                    }
                }
            }
        }
    }

    //we know that, for each color, there should be `count_per_color` units.
    //we also know the number of times each color appears as a known color
    //we'll use these pieces of info to build a new mapping of colors to the number of occurances
    //there must be as unknowns
    let mut unknown_color_counts = HashMap::new();
    for (&color, &known_count) in known_color_counts.iter() {
        let unknown_count = count_per_color
            .checked_sub(known_count)
            .expect("too many occurances of a single color");
        unknown_color_counts.insert(color, unknown_count);
    }

    //the values in our unknown color counts mapping will now sum to the number of unknown units
    //if-and-only-if it contains a key for every different color appearing in the state. if
    //the values don't sum to the number of unknown units, we know we are missing colors.
    //we just pick any not-yet-seen color, add an unknown count of `count_per_color`, and repeat
    //until everything adds up
    let present_color_unknown_count: usize = unknown_color_counts.values().sum();
    let mut missing_color_unknown_count = 0_usize;
    'outer: while (present_color_unknown_count + missing_color_unknown_count) < total_unknown_units
    {
        for color in ColoredWaterIter(None) {
            if let Entry::Vacant(e) = unknown_color_counts.entry(color) {
                e.insert(count_per_color);
                missing_color_unknown_count += count_per_color;
                continue 'outer;
            }
        }
        //if we reach here, it means we ran out of colors to add to our unknown_color_count
        //we can't really recover from this
        panic!("ran out of missing colors to add when trying to reach total_unknown_units");
    }
    //after breaking out of the loop, double-check that our new sum-of-unknown-counts exactly matches
    //the total number of unknown units
    assert_eq!(
        unknown_color_counts.values().sum::<usize>(),
        total_unknown_units,
        "sum of unknown color counts did not match total unknown units"
    );

    //we finally have what we need to build the vec: a list of colors mapped to their number of occurances.
    //build it and return
    let mut output = Vec::with_capacity(total_unknown_units);
    for (color, unknown_count) in unknown_color_counts.into_iter() {
        for _ in 0..unknown_count {
            output.push(color);
        }
    }
    Some(output)
}

/// Combine a [PartialGameState] and a permutable representation of its unknown colors into a [KnownGameState]
///
/// `permutable` must be the result of calling [to_permutable] with a copy of `gs`. `permutable` may be shuffled,
/// but should not have any elements added or removed. This property isn't checked, so be careful when calling this function
fn permutation_as_known<T, const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    mut gs: PartialGameState<MAX_BCOUNT, B_MAX_CAP>,
    mut permutable: T
) -> KnownGameState<MAX_BCOUNT, B_MAX_CAP>
where
    T: Iterator<Item = ColoredWaterUnit>
{
    //iterate through each unknown unit in each bottle,
    //setting each unknown unit to the "next" permutable representation item.
    for bottle in gs.bottles.iter_mut() {
        for c_idx in (0..bottle.capacity()).rev() {
            let sample_result = bottle.sample_at(c_idx);
            if sample_result != BottleSampleResult::UnknownColor {
                continue;
            }
            bottle
                .try_set_color(
                    c_idx,
                    Some(PartialColoredWaterUnit::Color(permutable.next().expect(
                        "permutable ran out of items before we ran out of unknown slots"
                    )))
                )
                .expect("failed to set color from permutable which should always succeed");
        }
    }

    gs.try_into()
        .expect("new state from permutable still had unknown units")
}
