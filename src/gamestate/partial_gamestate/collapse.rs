//! Logic for "collapsing" (as in "wave function collapse") one PartialGameState
//! into either all or a randome sample of possible KnownGameStates it could be

use std::collections::{hash_map::Entry, HashMap, HashSet};

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
    /// Tries to create and return a Vec of several possible [KnownGameState]s from this [PartialGameState]
    ///
    /// `max_count` specifies the maximum number of states to return. The returned Vec will never be longer than this,
    /// but may be shorter. No duplicates will be included in the return value.
    /// If this [PartialGameState] has more than `max_count` distinct possible [KnownGameState]s, a random sample of
    /// the states will be returned. If there are fewer than `max_count` distinct possible states, all states will be returned.
    ///
    /// This function will fail to produce a result if this [PartialGameState] has bottles of varying capacity. This limitation
    /// may or may not be resolved in the future.
    pub fn collapse(&self, max_count: usize) -> Option<Vec<KnownGameState<MAX_BCOUNT, B_MAX_CAP>>> {
        if let Some((permutable, perm_count)) = to_permutable(self) {
            // you might assume (as I did) that picking collapse_all here when max_count > perm_count would be most efficient.
            // however, it turns out that collapse_all is much, much slower than collapse_random_sample in terms of states per second,
            // so much so that using collapse_random_sample and hoping that you don't get too many duplicates ends up being as fast or faster
            // in all cases tested. therefore, we always use collapse_random_sample - the only difference based on perm_count is how we know when to stop.

            let found_states = if let Some(perm_count) = perm_count {
                // our actual state count will be either max_count or perm_count, whichever is smaller.
                // it's important we know which of the two limits to use; if we use max_count but perm_count is smaller,
                // we'll end up in an infinite loop here.
                let state_count = max_count.min(perm_count.try_into().unwrap_or(max_count));

                let mut found_states = HashSet::with_capacity(state_count);
                for possible_state in collapse_random_sample_iter(self, permutable) {
                    found_states.insert(possible_state);
                    if found_states.len() >= state_count {
                        break;
                    }
                }
                found_states.into_iter().collect()
            } else {
                //we couldn't calculate a perm count, and so can't guarantee we have at least max_count possible states.
                //although we almost certainly do have enough states, we want to avoid an infinite loop on the off chance we don't,
                //so we build in an attempt counter and emergency-bail-out at max_count * 4 iterations. This is almost certainly slower
                //and isn't guaranteed to return max_count states even if that is possible, but it at least avoids an infinite loop.
                let mut found_states = HashSet::with_capacity(max_count);
                let mut iters = 0_u128;
                let max_iters = (max_count as u128).saturating_mul(4);
                for possible_state in collapse_random_sample_iter(self, permutable) {
                    found_states.insert(possible_state);
                    iters += 1;
                    if found_states.len() >= max_count || iters >= max_iters {
                        break;
                    }
                }
                found_states.into_iter().collect()
            };
            Some(found_states)
        } else {
            None
        }
    }

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
        to_permutable(self).map(|permutable| collapse_all_iter(self, permutable.0))
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
        to_permutable(self).map(|permutable| collapse_random_sample_iter(self, permutable.0))
    }
}

/// Creates the iterator used by [PartialGameState::collapse_all] when given
/// a [PartialGameState] and the result of calling [to_permutable] on that state.
///
/// `as_permutable` must have come from calling [to_permutable] on `gs`;
/// bad things will happen if there's a mismatch, and a mismatch is not checked for.
/// This is only really useful on its own as an implementation detail of [PartialGameState::collapse].
fn collapse_all_iter<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gs: &PartialGameState<MAX_BCOUNT, B_MAX_CAP>,
    as_permutable: Vec<ColoredWaterUnit>
) -> impl Iterator<Item = KnownGameState<MAX_BCOUNT, B_MAX_CAP>> + use<'_, MAX_BCOUNT, B_MAX_CAP> {
    let length = as_permutable.len();
    as_permutable
        .into_iter()
        .permutations(length)
        .unique()
        .map(|permutation| permutation_as_known(gs.clone(), permutation.into_iter()))
}

/// Creates the iterator used by [PartialGameState::collapse_random_sample] when given
/// a [PartialGameState] and the result of calling [to_permutable] on that state.
///
/// `as_permutable` must have come from calling [to_permutable] on `gs`;
/// bad things will happen if there's a mismatch, and a mismatch is not checked for.
/// This is only really useful on its own as an implementation detail of [PartialGameState::collapse].
fn collapse_random_sample_iter<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gs: &PartialGameState<MAX_BCOUNT, B_MAX_CAP>,
    as_permutable: Vec<ColoredWaterUnit>
) -> impl Iterator<Item = KnownGameState<MAX_BCOUNT, B_MAX_CAP>> + use<'_, MAX_BCOUNT, B_MAX_CAP> {
    BasicShuffleIter::new(as_permutable)
        .map(|permutation| permutation_as_known(gs.clone(), permutation.into_iter()))
}

/// Generates a [Vec] of [ColoredWaterUnit]s from a [PartialGameState] and returns
/// the generated Vec alongside the number of unique permutations the Vec (if possible).
///
/// This vec contains one ColoredWaterUnit for each unknown color in the [PartialGameState]
/// It will have as many of each color as are "missing" from the [PartialGameState]. The vec
/// can then be shuffled around to generate valid possible KnownGameStates from the [PartialGameState].
///
/// The number of unique permutations is included as long as the total number of unknown units is less than 35.
/// The limit is set where it is because calculating the permutation count for 35 or more unknown units would require
/// handling an intermediate value of 35! or greater, which we can't easily do (since it exceeds [u128::MAX]). That said,
/// the highest number of unknown units for game states that will actually be seen in the mobile game should only be 33,
/// so this should typically be fine.
///
/// This function will return None if the [PartialGameState] has bottles of varying capacity, as it cannot be determined
/// conclusively which color has how many units.
/// TODO: fix this limitation if it's possible, or determine conclusively that it's not possible
fn to_permutable<const MAX_BCOUNT: usize, const B_MAX_CAP: usize>(
    gs: &PartialGameState<MAX_BCOUNT, B_MAX_CAP>
) -> Option<(Vec<ColoredWaterUnit>, Option<u128>)> {
    //try to get the capacity of the first bottle. if there is no first bottle, immediately return an empty vec
    let first_bottle_cap = gs.bottles.first().map(|b| b.capacity());
    if first_bottle_cap.is_none() {
        return Some((Vec::new(), Some(0)));
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

    //at this point, we have what we need to build the output vec, but we
    //first calculate the number of possible permutations if that's possible.
    //this is done with a formula of `n!/(a!*b!*c! ...)` where
    //`n` is the total number of units and `a`, `b`, `c` etc. represent the number of units
    //of each color.
    //see: https://math.stackexchange.com/questions/4038010/counting-permutation-of-duplicate-items
    let permutation_count = if total_unknown_units < 35 {
        let mut denominator = Some(1_u128);
        for &unknown_count in unknown_color_counts.values() {
            //unknown_count must be less than or equal to total_unknown_units,
            //and total_unknown_units is known to be under 35, so unknown_count must be under 35,
            //and this is safe to unwrap
            let this_count_factorial = factorial(unknown_count).unwrap();

            //I'm reasonably certain that there's no set of unknown counts where
            //the product of their factorials exceeds their sum's factorial, and since we already
            //ensured their sum's factorial is within bounds, the product of their factorials must also
            //be in bounds. That said, I'm not 100% certain, so we act as though bounds checking is necessary anyway.
            denominator = denominator.unwrap().checked_mul(this_count_factorial);
            if denominator.is_none() {
                break;
            }
        }

        denominator.map(|d| factorial(total_unknown_units).unwrap() / d)
    } else {
        None
    };

    //finally, build out vec and return
    let mut out_vec = Vec::with_capacity(total_unknown_units);
    for (color, unknown_count) in unknown_color_counts.into_iter() {
        for _ in 0..unknown_count {
            out_vec.push(color);
        }
    }

    Some((out_vec, permutation_count))
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

/// Returns n! if the value fits inside a u128 (i.e. `n! < 2^128`)
/// If the value does not fit inside a u128, returns None.
const fn factorial(n: usize) -> Option<u128> {
    // all factorials up to 34! will fit into a u128, and all factorials 35! and over will not.
    // therefore, there are only 35 possible inputs (including 0), which is few enough to justify using a look up table

    #[rustfmt::skip] //rustfmt really wants to make this table horrendously tall
    const FACTORIAL_TABLE: [u128; 35] = [
        1,
        1,
        2,
        3*2,
        4*3*2,
        5*4*3*2,
        6*5*4*3*2,
        7*6*5*4*3*2,
        8*7*6*5*4*3*2,
        9*8*7*6*5*4*3*2,
        10*9*8*7*6*5*4*3*2,
        11*10*9*8*7*6*5*4*3*2,
        12*11*10*9*8*7*6*5*4*3*2,
        13*12*11*10*9*8*7*6*5*4*3*2,
        14*13*12*11*10*9*8*7*6*5*4*3*2,
        15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        24*23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        25*24*23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        26*25*24*23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        27*26*25*24*23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        28*27*26*25*24*23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        29*28*27*26*25*24*23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        30*29*28*27*26*25*24*23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        31*30*29*28*27*26*25*24*23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        32*31*30*29*28*27*26*25*24*23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        33*32*31*30*29*28*27*26*25*24*23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2,
        34*33*32*31*30*29*28*27*26*25*24*23*22*21*20*19*18*17*16*15*14*13*12*11*10*9*8*7*6*5*4*3*2
    ];

    // we do this instead of `FACTORIAL_TABLE.get(n)` because `.get` isn't const,
    // but manual bounds checking and then plain-old-slice-indexing is const
    if n < 35 {
        Some(FACTORIAL_TABLE[n])
    } else {
        None
    }
}

#[cfg(test)]
mod factorial_test {
    use crate::gamestate::partial_gamestate::collapse::factorial;

    #[test]
    fn factorial_test() {
        //check expected values of 0!, 1!, 10!, 34!, and that 35! produces None
        assert_eq!(factorial(0), Some(1));
        assert_eq!(factorial(1), Some(1));
        assert_eq!(factorial(10), Some(3628800));
        assert_eq!(
            factorial(34),
            Some(295_232_799_039_604_140_847_618_609_643_520_000_000)
        );
        assert_eq!(factorial(35), None);
    }
}
