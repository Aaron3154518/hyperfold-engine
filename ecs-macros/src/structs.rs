use num_derive::FromPrimitive;
use syn;

// Hardcoded struct paths
pub const ENTITY_PATH: [&str; 4] = ["crate", "ecs", "entity", "Entity"];
pub const COMPONENTS_PATH: [&str; 4] = ["crate", "ecs", "component", "Components"];

// Labels
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

// Parsing macro args
enum MacroArgs {
    Dummy,
    Label,
    Const,
}

impl MacroArgs {
    fn from(vals: Vec<String>) -> Vec<Self> {
        vals.iter()
            .map(|s| match s.as_str() {
                "Dummy" => Self::Dummy,
                "Label" => Self::Label,
                "Const" => Self::Const,
                _ => panic!("Unknown Macro Arg: {}", s),
            })
            .collect()
    }
}

fn parse<T>(input: syn::parse::ParseStream) -> syn::Result<T>
where
    T: From<Vec<String>>,
{
    let mut args = Vec::new();
    while let Ok(i) = input.parse::<syn::Ident>() {
        args.push(i.to_string());
        let _ = input.parse::<syn::Token![,]>();
    }
    Ok(T::from(args))
}

// Component args
#[derive(Debug, Clone)]
pub struct ComponentMacroArgs {
    pub is_dummy: bool,
    pub is_label: bool,
}

impl From<Vec<String>> for ComponentMacroArgs {
    fn from(vals: Vec<String>) -> Self {
        let mut c = Self {
            is_dummy: false,
            is_label: false,
        };
        for a in MacroArgs::from(vals) {
            match a {
                MacroArgs::Dummy => c.is_dummy = true,
                MacroArgs::Label => c.is_label = true,
                MacroArgs::Const => {
                    panic!(
                        "{}\n{}",
                        "Component cannot be Const",
                        "Perhaps you meant to declare this as \"global\"?"
                    )
                }
            }
        }
        c
    }
}

impl syn::parse::Parse for ComponentMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        parse(input)
    }
}

// Global args
#[derive(Debug, Clone)]
pub struct GlobalMacroArgs {
    pub is_dummy: bool,
    pub is_const: bool,
}

impl From<Vec<String>> for GlobalMacroArgs {
    fn from(vals: Vec<String>) -> Self {
        let mut g = Self {
            is_dummy: false,
            is_const: false,
        };
        for a in MacroArgs::from(vals) {
            match a {
                MacroArgs::Dummy => g.is_dummy = true,
                MacroArgs::Const => g.is_const = true,
                MacroArgs::Label => panic!("Global cannot be a Label"),
            }
        }
        g
    }
}

impl syn::parse::Parse for GlobalMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        parse(input)
    }
}
