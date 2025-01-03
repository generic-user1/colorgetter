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
