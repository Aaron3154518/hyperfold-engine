use ecs_macros::shared::macro_args::{ComponentMacroArgs, GlobalMacroArgs};

use super::util::HasPath;

// Component
#[derive(Clone, Debug)]
pub struct Component {
    pub path: Vec<String>,
    pub args: ComponentMacroArgs,
}

impl Component {
    pub fn to_data(&self) -> String {
        self.get_path_str()
    }
}

impl HasPath for Component {
    fn get_path_str(&self) -> String {
        self.path.join("::")
    }
}

// Global
#[derive(Clone, Debug)]
pub struct Global {
    pub path: Vec<String>,
    pub args: GlobalMacroArgs,
}

impl Global {
    pub fn to_data(&self) -> String {
        self.get_path_str()
    }
}

impl HasPath for Global {
    fn get_path_str(&self) -> String {
        self.path.join("::")
    }
}
