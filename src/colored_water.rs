//! Implementation for the colored water that goes into [Bottle](crate::bottle::Bottle)
use crossterm::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A group of consecutive [ColoredWaterUnit]s that are all the same color
pub struct ColoredWaterRun {
    pub color: ColoredWaterUnit,
    pub size: usize
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// One unit of colored water, with a specific color
pub enum ColoredWaterUnit {
    Red,
    Maroon,
    Lime,
    Green,
    Aqua,
    Blue,
    Yellow,
    Orange,
    Pink,
    Tan,
    Brown
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
        value.into()
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
