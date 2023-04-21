use num_derive::FromPrimitive;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromPrimitive)]
pub enum ComponentArgs {
    None,
    Global,
}
