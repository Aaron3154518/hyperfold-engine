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

    pub fn from_data(v: &str) -> Option<Self> {
        match v {
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
        for v in vals {
            match v.as_str() {
                "Dummy" => c.is_dummy = true,
                "Label" => c.is_label = true,
                "Const" => panic!(
                    "{}\n{}",
                    "Component cannot be Const", "Perhaps you meant to declare this as \"global\"?"
                ),
                _ => panic!("Unknown macro argument for component: {}", v),
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
        for v in vals {
            match v.as_str() {
                "Dummy" => g.is_dummy = true,
                "Const" => g.is_const = true,
                _ => panic!("Unknown macro argument for global: {}", v),
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

// System args
#[derive(Debug, Clone)]
pub struct SystemMacroArgs {
    pub is_init: bool,
}

impl From<Vec<String>> for SystemMacroArgs {
    fn from(vals: Vec<String>) -> Self {
        let mut s = Self { is_init: false };
        for v in vals {
            match v.as_str() {
                "Init" => s.is_init = true,
                _ => panic!("Unknown macro argument for system: {}", v),
            }
        }
        s
    }
}

impl syn::parse::Parse for SystemMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        parse(input)
    }
}
