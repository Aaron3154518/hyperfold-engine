use hyperfold_shared::macro_args::{ComponentMacroArgs, GlobalMacroArgs};

use crate::util::{concatenate, AddModPath};

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

impl AddModPath for Component {
    fn add_mod_path(&mut self, path: Vec<String>) {
        self.path = concatenate(path, self.path.to_vec())
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

impl AddModPath for Global {
    fn add_mod_path(&mut self, path: Vec<String>) {
        self.path = concatenate(path, self.path.to_vec())
    }
}

impl HasPath for Global {
    fn get_path_str(&self) -> String {
        self.path.join("::")
    }
}

// Trait
#[derive(Clone, Debug)]
pub struct Trait {
    pub g_trait: Global,
    pub global: Global,
}
