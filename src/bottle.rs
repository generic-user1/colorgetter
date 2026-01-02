//! Implementation of bottles for colored water

// linter thinks this is unused, but it actually is used in the macro definitions
#[allow(unused_imports)]
use crate::colored_water::ColoredWaterUnit;

mod misc;
pub use misc::{BottleCapacityError, BottleMaxCapError, PourInError, PourOutError};

mod bottle_trait;
pub use bottle_trait::{Bottle, BottleSampleConversionError, BottleSampleResult, ColorSetError};

mod known_bottle;
pub use known_bottle::{KnownBottle, PartialBottleConversionError};

mod partial_bottle;
pub use partial_bottle::PartialBottle;

#[cfg(test)]
mod bottle_tests;

/// Create a KnownBottle with some content and an optional size
///
/// There are four forms:
///
/// - Defining a bottle with its content, a capacity, and an explicit max capacity:
///   `bottle!([<Color>, <Color>, ...], <capacity>, <max capacity>)`
///
/// - Definining a bottle with its content and a capacity:
///   `bottle!([<Color>, <Color>, ...], <capacity>)`
///
/// - Defining a bottle with its content and allowing its capacity to match:
///   `bottle!([<Color>, <Color>, ...])`
///
/// - Definining a bottle with its content and allowing its capacity to match,
///   omitting square brackets `[]` surrounding color definitions:
///   `bottle!(<Color>, <Color>, ...)`
///
/// In each form, `<Color>` may be any variant of [ColoredWaterUnit], though
/// you can (and in fact must) omit the `ColoredWaterUnit::` snippet.
/// As few as 1 ColoredWaterUnits may be used, and there is no upper bound.
///
/// Examples:
/// ```
/// use colorgetter::bottle;
/// use colorgetter::bottle::{KnownBottle, Bottle};
/// use colorgetter::colored_water::ColoredWaterUnit;
///
/// // Bottle defined with content, explicit capacity, and explicit max capacity (allows us to forgo type hinting our variable)
/// let sized_bottle1 = bottle!([Red, Green, Yellow], 4, 5);
/// assert_eq!(
///     *sized_bottle1.get_content(),
///     [ColoredWaterUnit::Red, ColoredWaterUnit::Green, ColoredWaterUnit::Yellow]
/// );
///
/// assert_eq!(sized_bottle1.capacity(), 4);
/// assert_eq!(sized_bottle1.get_max_capacity(), 5);
///
/// // Bottle defined with content and explicit capacity
/// let sized_bottle2: KnownBottle<4> = bottle!([Red, Green, Yellow], 4);
/// assert_eq!(
///     *sized_bottle2.get_content(),
///     [ColoredWaterUnit::Red, ColoredWaterUnit::Green, ColoredWaterUnit::Yellow]
/// );
/// assert_eq!(sized_bottle2.capacity(), 4);
///
/// // Bottle defined with content only
/// let unsized_bottle1: KnownBottle<4> = bottle!([Red, Green, Yellow]);
/// assert_eq!(
///     *unsized_bottle1.get_content(),
///     [ColoredWaterUnit::Red, ColoredWaterUnit::Green, ColoredWaterUnit::Yellow]
/// );
/// assert_eq!(unsized_bottle1.capacity(), 3);
///
/// // Bottle defined with content only, omitting square brackets
/// let unsized_bottle2: KnownBottle<4> = bottle!(Red, Green, Yellow);
/// assert_eq!(
///     *unsized_bottle2.get_content(),
///     [ColoredWaterUnit::Red, ColoredWaterUnit::Green, ColoredWaterUnit::Yellow]
/// );
/// assert_eq!(unsized_bottle2.capacity(), 3);
/// ```
#[macro_export]
macro_rules! bottle {
    ([$($color:ident),*], $capacity:expr, $max_capacity:expr) => {
        KnownBottle::<$max_capacity>::try_with_content(&[$(ColoredWaterUnit::$color),*]).unwrap().try_take_as_resized($capacity).unwrap()
    };
    ([$($color:ident),*], $capacity:expr) => {
        KnownBottle::try_with_content(&[$(ColoredWaterUnit::$color),*]).unwrap().try_take_as_resized($capacity).unwrap()
    };
    ([$($color:ident),+]) => {
        KnownBottle::try_with_content(&[$(ColoredWaterUnit::$color),+]).unwrap()
    };
    ($($color:ident),+) => {
        bottle!([$($color),+])
    }
}

/// Create an array of [ColoredWaterUnit]s in a more compact form;
/// useful for quickly defining the content of a KnownBottle.
///
/// To match [bottle!], comes in 2 forms:
///
/// - Defining water colors with square brackets:
///   `bottle_content!([<Color>, <Color>, ...])`
///
/// - Defining water colors without square brackets:
///   `bottle_content!(<Color>, <Color>, ...)`
///
/// In each form, `<Color>` may be any variant of [ColoredWaterUnit], though
/// you can (and in fact must) omit the `ColoredWaterUnit::` snippet.
/// As few as 1 ColoredWaterUnits may be used, and there is no upper bound.
///
/// Examples:
/// ```
/// use colorgetter::bottle_content;
/// use colorgetter::colored_water::ColoredWaterUnit;
///
/// let first_form = bottle_content!([Red, Green, Yellow]);
/// assert_eq!(
///     first_form,
///     [ColoredWaterUnit::Red, ColoredWaterUnit::Green, ColoredWaterUnit::Yellow]
/// );
///
/// let second_form = bottle_content!(Orange, Lime, Pink);
/// assert_eq!(
///     second_form,
///     [ColoredWaterUnit::Orange, ColoredWaterUnit::Lime, ColoredWaterUnit::Pink]
/// );
/// ```
#[macro_export]
macro_rules! bottle_content {
    ($($color:ident),+) => {
        [$(ColoredWaterUnit::$color),+]
    };
    ([$($color:ident),+]) => {
        [$(ColoredWaterUnit::$color),+]
    };
}
