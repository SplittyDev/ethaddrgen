use clap::ValueEnum;
use termcolor::ColorChoice;

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum ColorValue {
    Always,
    AlwaysAnsi,
    Auto,
    Never,
}

impl Default for ColorValue {
    fn default() -> Self {
        Self::Auto
    }
}

impl From<ColorValue> for ColorChoice {
    fn from(value: ColorValue) -> Self {
        match value {
            ColorValue::Always => Self::Always,
            ColorValue::AlwaysAnsi => Self::AlwaysAnsi,
            ColorValue::Auto => Self::Auto,
            ColorValue::Never => Self::Never,
        }
    }
}