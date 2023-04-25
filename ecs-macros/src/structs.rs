use num_derive::FromPrimitive;
use syn;

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

#[derive(Debug)]
pub struct ComponentType {
    types: Vec<ComponentArgs>,
}

impl ComponentType {
    pub fn is_dummy(&self) -> bool {
        self.types.contains(&ComponentArgs::Dummy)
    }

    pub fn is_global(&self) -> bool {
        self.types.contains(&ComponentArgs::Global)
    }
}

impl syn::parse::Parse for ComponentType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = Vec::new();
        while let Ok(i) = input.parse::<syn::Ident>() {
            args.push(i);
            let _ = input.parse::<syn::Token![,]>();
        }
        Ok(Self {
            types: args
                .iter()
                .map(|i| ComponentArgs::from(i.to_string().as_str()))
                .collect(),
        })
    }
}
