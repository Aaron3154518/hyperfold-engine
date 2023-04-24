use num_derive::FromPrimitive;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromPrimitive)]
pub enum ComponentArgs {
    None,
    Global,
    Dummy,
}

impl From<&str> for ComponentArgs {
    fn from(value: &str) -> Self {
        match value {
            "Global" => Self::Global,
            "Dummy" => Self::Dummy,
            _ => Self::None,
        }
    }
}
