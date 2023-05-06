use crate::util::{concatenate, AddModPath};

use super::util::HasPath;

// Event
#[derive(Clone, Debug)]
pub struct EventMod {
    pub path: Vec<String>,
    pub events: Vec<String>,
}

impl EventMod {
    pub fn to_data(&self) -> String {
        format!(
            "{}({})",
            self.path.join("::"),
            self.events
                .iter()
                .map(|s| format!("{}", s))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

impl HasPath for EventMod {
    fn get_path_str(&self) -> String {
        self.path.join("::")
    }
}

impl AddModPath for EventMod {
    fn add_mod_path(&mut self, path: Vec<String>) {
        self.path = concatenate(path, self.path.to_vec())
    }
}
