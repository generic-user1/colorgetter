//! Implementation for the colored water that goes into [Bottle](crate::bottle::Bottle)
use crossterm::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A group of consecutive [ColoredWaterUnit]s that are all the same color
pub struct ColoredWaterRun {
    pub color: ColoredWaterUnit,
    pub size: usize
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
/// One unit of colored water, with a specific color
pub enum ColoredWaterUnit {
    Red = 0,
    Maroon = 1,
    Lime = 2,
    Green = 3,
    Aqua = 4,
    Blue = 5,
    Yellow = 6,
    Orange = 7,
    Pink = 8,
    Purple = 9,
    Tan = 10,
    Brown = 11
}

impl ColoredWaterUnit {
    /// Return the 'next' variant, or [None] if there is no next variant
    pub fn next(&self) -> Option<ColoredWaterUnit> {
        match self {
            Self::Red => Some(Self::Maroon),
            Self::Maroon => Some(Self::Lime),
            Self::Lime => Some(Self::Green),
            Self::Green => Some(Self::Aqua),
            Self::Aqua => Some(Self::Blue),
            Self::Blue => Some(Self::Yellow),
            Self::Yellow => Some(Self::Orange),
            Self::Orange => Some(Self::Pink),
            Self::Pink => Some(Self::Purple),
            Self::Purple => Some(Self::Tan),
            Self::Tan => Some(Self::Brown),
            Self::Brown => None
        }
    }

    /// Return the 'previous' variant, or [None] if there is no previous variant
    pub fn prev(&self) -> Option<ColoredWaterUnit> {
        match self {
            Self::Red => None,
            Self::Maroon => Some(Self::Red),
            Self::Lime => Some(Self::Maroon),
            Self::Green => Some(Self::Lime),
            Self::Aqua => Some(Self::Green),
            Self::Blue => Some(Self::Aqua),
            Self::Yellow => Some(Self::Blue),
            Self::Orange => Some(Self::Yellow),
            Self::Pink => Some(Self::Orange),
            Self::Purple => Some(Self::Pink),
            Self::Tan => Some(Self::Purple),
            Self::Brown => Some(Self::Tan)
        }
    }

    /// Return the 'first' variant
    pub fn first() -> ColoredWaterUnit {
        Self::Red
    }

    /// Return the 'last' variant
    pub fn last() -> ColoredWaterUnit {
        Self::Brown
    }
}

impl From<ColoredWaterRun> for Vec<ColoredWaterUnit> {
    fn from(value: ColoredWaterRun) -> Self {
        let mut out = Vec::with_capacity(value.size);
        for _ in 0..value.size {
            out.push(value.color);
        }
        out
    }
}

impl From<&ColoredWaterUnit> for crossterm::style::Color {
    fn from(value: &ColoredWaterUnit) -> Self {
        match value {
            ColoredWaterUnit::Red => Color::Rgb {
                r: 0xFF,
                g: 0x00,
                b: 0x00
            },
            ColoredWaterUnit::Maroon => Color::Rgb {
                r: 0x80,
                g: 0x00,
                b: 0x00
            },
            ColoredWaterUnit::Lime => Color::Rgb {
                r: 0x00,
                g: 0xFF,
                b: 0x00
            },
            ColoredWaterUnit::Green => Color::Rgb {
                r: 0x00,
                g: 0x80,
                b: 0x00
            },
            ColoredWaterUnit::Aqua => Color::Rgb {
                r: 0x00,
                g: 0xFF,
                b: 0xFF
            },
            ColoredWaterUnit::Blue => Color::Rgb {
                r: 0x00,
                g: 0x00,
                b: 0xFF
            },
            ColoredWaterUnit::Yellow => Color::Rgb {
                r: 0xFF,
                g: 0xD7,
                b: 0x00
            },
            ColoredWaterUnit::Orange => Color::Rgb {
                r: 0xFE,
                g: 0x8A,
                b: 0x18
            },
            ColoredWaterUnit::Pink => Color::Rgb {
                r: 0xFF,
                g: 0x69,
                b: 0xB4
            },
            ColoredWaterUnit::Purple => Color::Rgb {
                r: 70,
                g: 20,
                b: 101
            },
            ColoredWaterUnit::Tan => Color::Rgb {
                r: 0xD2,
                g: 0xB4,
                b: 0x8C
            },
            ColoredWaterUnit::Brown => Color::Rgb {
                r: 0x8B,
                g: 0x45,
                b: 0x13
            }
        }
    }
}

impl From<ColoredWaterUnit> for crossterm::style::Color {
    fn from(value: ColoredWaterUnit) -> Self {
        (&value).into()
    }
}

/// Reasons converting from a `&[ColoredWaterUnit]` may fail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColoredWaterRunError {
    /// The source is empty and it is therefore impossible to know what color to use
    Empty,

    /// The given source has multiple colors
    MismatchedColors
}

impl<const N: usize> TryFrom<[ColoredWaterUnit; N]> for ColoredWaterRun {
    type Error = ColoredWaterRunError;
    fn try_from(value: [ColoredWaterUnit; N]) -> Result<Self, Self::Error> {
        ColoredWaterRun::try_from(&value)
    }
}

impl<const N: usize> TryFrom<&[ColoredWaterUnit; N]> for ColoredWaterRun {
    type Error = ColoredWaterRunError;
    fn try_from(value: &[ColoredWaterUnit; N]) -> Result<Self, Self::Error> {
        ColoredWaterRun::try_from(value.as_ref())
    }
}

impl<const MAX_CAP: usize> TryFrom<&heapless::Vec<ColoredWaterUnit, MAX_CAP>> for ColoredWaterRun {
    type Error = ColoredWaterRunError;
    fn try_from(value: &heapless::Vec<ColoredWaterUnit, MAX_CAP>) -> Result<Self, Self::Error> {
        ColoredWaterRun::try_from(value as &[ColoredWaterUnit])
    }
}

impl TryFrom<&[ColoredWaterUnit]> for ColoredWaterRun {
    type Error = ColoredWaterRunError;
    fn try_from(value: &[ColoredWaterUnit]) -> Result<Self, Self::Error> {
        let mut encountered_color = None;
        for &color_unit in value {
            match encountered_color {
                None => {
                    encountered_color = Some(color_unit);
                }
                Some(encountered_color) if encountered_color != color_unit => {
                    return Err(ColoredWaterRunError::MismatchedColors);
                }
                _ => ()
            }
        }
        if let Some(color) = encountered_color {
            Ok(ColoredWaterRun {
                color,
                size: value.len()
            })
        } else {
            Err(ColoredWaterRunError::Empty)
        }
    }
}

/// A [ColoredWaterUnit] that may be unknown, used with [PartialBottle](crate::bottle::PartialBottle)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartialColoredWaterUnit {
    /// This unit has a known color (includes the specific color)
    Color(ColoredWaterUnit),
    /// This unit has a color, but which color specifically is not known
    UnknownColor
}

impl From<ColoredWaterUnit> for PartialColoredWaterUnit {
    fn from(value: ColoredWaterUnit) -> Self {
        PartialColoredWaterUnit::Color(value)
    }
}

impl TryFrom<PartialColoredWaterUnit> for ColoredWaterUnit {
    type Error = PartialColorConversionError;
    fn try_from(value: PartialColoredWaterUnit) -> Result<Self, Self::Error> {
        match value {
            PartialColoredWaterUnit::Color(v) => Ok(v),
            PartialColoredWaterUnit::UnknownColor => Err(PartialColorConversionError::UnknownColor)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A group of consecutive [PartialColoredWaterUnit]s that are all the same color
/// or all unknown
pub struct PartialColoredWaterRun {
    pub color: PartialColoredWaterUnit,
    pub size: usize
}

impl From<ColoredWaterRun> for PartialColoredWaterRun {
    fn from(value: ColoredWaterRun) -> Self {
        PartialColoredWaterRun {
            color: PartialColoredWaterUnit::Color(value.color),
            size: value.size
        }
    }
}

impl TryFrom<PartialColoredWaterRun> for ColoredWaterRun {
    type Error = PartialColorConversionError;
    fn try_from(value: PartialColoredWaterRun) -> Result<Self, Self::Error> {
        match value {
            PartialColoredWaterRun {
                color: PartialColoredWaterUnit::Color(color),
                size
            } => Ok(ColoredWaterRun { color, size }),
            PartialColoredWaterRun {
                color: PartialColoredWaterUnit::UnknownColor,
                ..
            } => Err(PartialColorConversionError::UnknownColor)
        }
    }
}

/// Reasons converting from a [PartialColoredWaterUnit] to a [ColoredWaterUnit]
/// or from a [PartialColoredWaterRun] to a [ColoredWaterRun] may fail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartialColorConversionError {
    /// The [PartialColoredWaterUnit] is unknown
    UnknownColor
}
