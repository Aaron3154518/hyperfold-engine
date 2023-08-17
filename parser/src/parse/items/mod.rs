use proc_macro2::Span;

mod enums;
mod functions;
mod macros;
mod structs;
mod uses;

pub use enums::AstEnum;
pub use functions::AstFunction;
pub use macros::AstMacroCall;
pub use structs::AstStruct;
pub use uses::AstUse;

use super::AstAttribute;

// Data used for all Ast Items
#[derive(Debug, Clone)]
pub struct AstItemData {
    // Includes ident
    pub path: Vec<String>,
    pub ident: String,
    pub span: Span,
}

#[derive(Debug)]
pub struct AstItems {
    pub structs: Vec<AstStruct>,
    pub enums: Vec<AstEnum>,
    pub functions: Vec<AstFunction>,
    pub macro_calls: Vec<AstMacroCall>,
}

impl AstItems {
    pub fn new() -> Self {
        Self {
            structs: Vec::new(),
            enums: Vec::new(),
            functions: Vec::new(),
            macro_calls: Vec::new(),
        }
    }

    pub fn structs_and_enums(&self) -> impl Iterator<Item = (&AstItemData, &Vec<AstAttribute>)> {
        self.structs
            .iter()
            .map(|s| (&s.data, &s.attrs))
            .chain(self.enums.iter().map(|e| (&e.data, &e.attrs)))
    }
}
