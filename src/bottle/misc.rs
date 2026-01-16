//! Miscellaneous things related to Bottles
use std::fmt::Display;

/// All reasons why creating or resizing a [KnownBottle](super::KnownBottle) or [PartialBottle](super::PartialBottle) may fail
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BottleCapacityError {
    /// The capacity requested is greater than the `MAX_CAP` of the Bottle
    MaxCapExceeded,

    /// The bottle is required to have more content than it has capacity. For [KnownBottle](super::KnownBottle),
    /// this can only occur when deserializing. For [PartialBottle](super::PartialBottle), this may occur during
    /// deserialization, but can also occur due to invalid combinations of `capacity` and `unknown_count`.
    CapExceeded
}
impl Display for BottleCapacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BottleCapacityError::MaxCapExceeded => write!(f, "Bottle Maximum Capacity Exceeded"),
            BottleCapacityError::CapExceeded => write!(f, "Bottle Capacity Exceeded")
        }
    }
}

/// Subset of [BottleCapacityError] including only errors that can occur
/// during normal use of [KnownBottle](super::KnownBottle) (i.e. excluding deserialization and [PartialBottle](super::PartialBottle))
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BottleMaxCapError {
    /// The capacity requested is greater than the `MAX_CAP` of the Bottle
    MaxCapExceeded
}

impl From<BottleMaxCapError> for BottleCapacityError {
    fn from(value: BottleMaxCapError) -> Self {
        match value {
            BottleMaxCapError::MaxCapExceeded => BottleCapacityError::MaxCapExceeded
        }
    }
}

///Reasons that pouring a [ColoredWaterRun](crate::colored_water::ColoredWaterRun) into a [Bottle](super::Bottle) may fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PourInError {
    /// The destination [Bottle](super::Bottle) is full and cannot accept any part of the [ColoredWaterRun](crate::colored_water::ColoredWaterRun)
    AlreadyFull,

    /// The destination [Bottle](super::Bottle) has a top color that does not match the color of the [ColoredWaterRun](crate::colored_water::ColoredWaterRun)
    MismatchedColors,

    /// The destination [Bottle](super::Bottle) has an unknown top color
    DestUnknownColor
}

///Reasons that pouring a [ColoredWaterRun](crate::colored_water::ColoredWaterRun) out of a [KnownBottle](super::KnownBottle)  may fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PourOutError {
    /// The source [Bottle](super::Bottle) is entirely empty and has no content to pour
    Empty,

    /// The source [Bottle](super::Bottle) has an unknown top color
    SourceUnknownColor,

    /// The destination [Bottle](super::Bottle) could not accept the content to pour
    DestinationError(PourInError)
}

impl From<PourInError> for PourOutError {
    fn from(value: PourInError) -> Self {
        PourOutError::DestinationError(value)
    }
}
