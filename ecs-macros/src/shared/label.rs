// Label paths
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
