use num_derive::FromPrimitive;
use syn;

// Hardcoded struct paths
pub const ENTITY_PATH: [&str; 4] = ["crate", "ecs", "entity", "Entity"];
pub const COMPONENTS_PATH: [&str; 4] = ["crate", "ecs", "component", "Components"];
pub const LABEL_PATH: [&str; 4] = ["crate", "ecs", "component", "Label"];
pub const AND_LABELS_PATH: [&str; 4] = ["crate", "ecs", "component", "AndLabels"];
pub const OR_LABELS_PATH: [&str; 4] = ["crate", "ecs", "component", "OrLabels"];
pub const NAND_LABELS_PATH: [&str; 4] = ["crate", "ecs", "component", "NandLabels"];
pub const NOR_LABELS_PATH: [&str; 4] = ["crate", "ecs", "component", "NorLabels"];

#[derive(Clone, Debug)]
pub enum LabelType {
    And,
    Or,
    Nand,
    Nor,
}

pub const NUM_LABEL_TYPES: usize = 4;

impl LabelType {
    pub fn regex() -> &'static str {
        r"!?(&|\|)"
    }

    pub fn to_data(&self) -> &str {
        match self {
            LabelType::And => "&",
            LabelType::Or => "|",
            LabelType::Nand => "!&",
            LabelType::Nor => "!|",
        }
    }

    pub fn from_data(s: &str) -> Option<Self> {
        match s {
            "&" => Some(Self::And),
            "|" => Some(Self::Or),
            "!&" => Some(Self::Nand),
            "!|" => Some(Self::Nor),
            _ => None,
        }
    }

    pub fn from(path: &Vec<String>) -> Option<Self> {
        if *path == AND_LABELS_PATH {
            Some(Self::And)
        } else if *path == OR_LABELS_PATH {
            Some(Self::Or)
        } else if *path == NAND_LABELS_PATH {
            Some(Self::Nand)
        } else if *path == NOR_LABELS_PATH {
            Some(Self::Nor)
        } else {
            None
        }
    }
}

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
    pub is_label: bool,
}

impl From<Vec<String>> for ComponentType {
    fn from(value: Vec<String>) -> Self {
        let mut c = Self {
            ty: ComponentTypes::None,
            is_dummy: false,
            is_label: false,
        };
        for s in value.iter() {
            match s.as_str() {
                "Global" => c.ty = ComponentTypes::Global,
                "Dummy" => c.is_dummy = true,
                "Label" => c.is_label = true,
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
