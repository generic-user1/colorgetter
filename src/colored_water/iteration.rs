use super::{ColoredWaterUnit, PartialColoredWaterUnit};

/// Iterator over possible values of [ColoredWaterUnit]
///
/// The only field tracks the color last-yielded by the iterator. It
/// may be [None] - when it is, the next color to be yielded will be [ColoredWaterUnit::first].
/// In other words, this iterator resumes iteration after it has ended.
pub struct ColoredWaterIter(pub Option<ColoredWaterUnit>);

impl Iterator for ColoredWaterIter {
    type Item = ColoredWaterUnit;
    fn next(&mut self) -> Option<Self::Item> {
        self.0 = if let Some(last_color) = self.0 {
            last_color.next()
        } else {
            Some(ColoredWaterUnit::first())
        };
        self.0
    }
}

impl DoubleEndedIterator for ColoredWaterIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0 = if let Some(last_color) = self.0 {
            last_color.prev()
        } else {
            Some(ColoredWaterUnit::last())
        };
        self.0
    }
}

/// Iterator over possible values of [PartialColoredWaterUnit]
///
/// The only field tracks the color last-yielded by the iterator. It
/// may be [None] - when it is, the next color to be yielded will be [PartialColoredWaterUnit::first].
/// In other words, this iterator resumes iteration after it has ended.
pub struct PartialColoredWaterIter(pub Option<PartialColoredWaterUnit>);

impl Iterator for PartialColoredWaterIter {
    type Item = PartialColoredWaterUnit;
    fn next(&mut self) -> Option<Self::Item> {
        self.0 = if let Some(last_color) = self.0 {
            last_color.next()
        } else {
            Some(PartialColoredWaterUnit::first())
        };
        self.0
    }
}

impl DoubleEndedIterator for PartialColoredWaterIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0 = if let Some(last_color) = self.0 {
            last_color.prev()
        } else {
            Some(PartialColoredWaterUnit::last())
        };
        self.0
    }
}
