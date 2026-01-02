use super::*;
use crate::bottle::{PartialBottle, PartialBottleConversionError};
use heapless::{CapacityError, Vec};
use serde::{Deserialize, Serialize};

/// The state a particular game is in, including [PartialBottle]s
/// instead of regular [KnownBottle](crate::bottle::KnownBottle)s
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartialGameState<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> {
    pub bottles: Vec<PartialBottle<B_MAX_CAP>, MAX_BCOUNT>
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

impl<const MAX_BCOUNT: usize, const B_MAX_CAP: usize> GameStateDisplay
    for PartialGameState<MAX_BCOUNT, B_MAX_CAP>
{
    type BottleT = PartialBottle<B_MAX_CAP>;

    fn get_bottles(&self) -> &[Self::BottleT] {
        &self.bottles
    }
}
