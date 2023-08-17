use shared::parsing::{ComponentMacroArgs, GlobalMacroArgs};

use crate::parse::ItemPath;

use super::items::ItemData;

#[derive(Debug)]
pub struct ItemComponent {
    pub data: ItemData,
    pub args: ComponentMacroArgs,
}

#[derive(Clone, Debug)]
pub struct ItemGlobal {
    pub data: ItemData,
    pub args: GlobalMacroArgs,
}

#[derive(Clone, Debug)]
pub struct ItemTrait {
    pub data: ItemData,
    pub g_idx: usize,
}

#[derive(Debug)]
pub struct ItemEvent {
    pub data: ItemData,
    pub state: Option<usize>,
}

#[derive(Debug)]
pub struct ItemState {
    pub data: ItemData,
    pub data_path: ItemPath,
    pub enter_event: usize,
    pub exit_event: usize,
    pub label: usize,
}
