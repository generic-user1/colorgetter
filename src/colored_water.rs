//! Implementation for the colored water that goes into [Bottle](crate::bottle::Bottle)

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

#[macro_export]
macro_rules! bottle_content {
    ($($color:tt),+) => {
        [$(ColoredWaterUnit::$color),+]
    };
}

/// Reasons converting from a `&[ColoredWaterUnit]` may fail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColoredWaterRunError {
    /// The source is empty and it is therefore impossible to know what color to use
    Empty,

    /// The given source has multiple colors
    MismatchedColors
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

#[derive(Debug, Clone, Copy)]
/// A color represented in RGB format
///
/// Can be used to represent the actual RGB color of an instance of [ColoredWaterUnit]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8
}

impl From<ColoredWaterUnit> for RgbColor {
    fn from(value: ColoredWaterUnit) -> Self {
        match value {
            ColoredWaterUnit::Red => RgbColor {
                red: 0xFF,
                green: 0x00,
                blue: 0x00
            },
            ColoredWaterUnit::Maroon => RgbColor {
                red: 0x80,
                green: 0x00,
                blue: 0x00
            },
            ColoredWaterUnit::Lime => RgbColor {
                red: 0x00,
                green: 0xFF,
                blue: 0x00
            },
            ColoredWaterUnit::Green => RgbColor {
                red: 0x00,
                green: 0x80,
                blue: 0x00
            },
            ColoredWaterUnit::Aqua => RgbColor {
                red: 0x00,
                green: 0xFF,
                blue: 0xFF
            },
            ColoredWaterUnit::Blue => RgbColor {
                red: 0x00,
                green: 0x00,
                blue: 0xFF
            },
            ColoredWaterUnit::Yellow => RgbColor {
                red: 0xFF,
                green: 0xD7,
                blue: 0x00
            },
            ColoredWaterUnit::Orange => RgbColor {
                red: 0xFF,
                green: 0x45,
                blue: 0x00
            },
            ColoredWaterUnit::Pink => RgbColor {
                red: 0xFF,
                green: 0x69,
                blue: 0xB4
            },
            ColoredWaterUnit::Tan => RgbColor {
                red: 0xD2,
                green: 0xB4,
                blue: 0x8C
            },
            ColoredWaterUnit::Brown => RgbColor {
                red: 0x8B,
                green: 0x45,
                blue: 0x13
            }
        }
    }
}
