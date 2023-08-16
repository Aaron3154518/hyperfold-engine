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

// Data used for all Ast Items
#[derive(Debug)]
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
}
