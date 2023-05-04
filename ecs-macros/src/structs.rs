use num_derive::FromPrimitive;
use syn;

// Hardcoded struct paths
pub const ENTITY_PATH: [&str; 4] = ["crate", "ecs", "entity", "Entity"];
pub const COMPONENTS_PATH: [&str; 4] = ["crate", "ecs", "component", "Components"];
pub const LABELS_PATH: [&str; 4] = ["crate", "ecs", "component", "Labels"];

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromPrimitive)]
pub enum ComponentTypes {
    None,
    Global,
}

#[derive(Debug)]
pub struct ComponentType {
    pub ty: ComponentTypes,
    pub is_dummy: bool,
}

impl From<Vec<String>> for ComponentType {
    fn from(value: Vec<String>) -> Self {
        let mut c = Self {
            ty: ComponentTypes::None,
            is_dummy: false,
        };
        for s in value.iter() {
            match s.as_str() {
                "Global" => c.ty = ComponentTypes::Global,
                "Dummy" => c.is_dummy = true,
                _ => (),
            }
        }
        c
    }
}

impl syn::parse::Parse for ComponentType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = Vec::new();
        while let Ok(i) = input.parse::<syn::Ident>() {
            args.push(i.to_string());
            let _ = input.parse::<syn::Token![,]>();
        }
        Ok(Self::from(args))
    }
}
