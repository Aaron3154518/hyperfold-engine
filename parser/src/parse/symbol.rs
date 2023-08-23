use diagnostic::ToErr;
use proc_macro2::Span;
use std::{cmp::Eq, fmt::Display, hash::Hash};

use syn::spanned::Spanned;

use crate::utils::paths::{CratePath, ENGINE_PATHS, MACRO_PATHS};

use super::AstMod;

use shared::{
    macros::{expand_enum, ExpandEnum},
    parsing::{ComponentMacroArgs, GlobalMacroArgs},
    syn::error::{CriticalResult, Error, StrToError},
    traits::CollectVec,
};

// Symbol with path - Edit this to add new engine items
#[derive(Eq, PartialEq)]
#[expand_enum]
pub enum HardcodedSymbol {
    // Macros crate
    ComponentMacro,
    GlobalMacro,
    EventMacro,
    StateMacro,
    SystemMacro,
    // Engine crate
    ComponentsMacro,
}

impl HardcodedSymbol {
    pub fn get_path(&self) -> &CratePath {
        match self {
            HardcodedSymbol::ComponentMacro => &MACRO_PATHS.component,
            HardcodedSymbol::GlobalMacro => &MACRO_PATHS.global,
            HardcodedSymbol::EventMacro => &MACRO_PATHS.event,
            HardcodedSymbol::StateMacro => &MACRO_PATHS.state,
            HardcodedSymbol::SystemMacro => &MACRO_PATHS.system,
            HardcodedSymbol::ComponentsMacro => &MACRO_PATHS.components,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ComponentSymbol {
    pub idx: usize,
    pub args: ComponentMacroArgs,
    pub span: Span,
}

// Used to index with labels
impl Hash for ComponentSymbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
    }
}

impl PartialEq for ComponentSymbol {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}

impl Eq for ComponentSymbol {}

#[derive(Copy, Clone, Debug)]
pub struct GlobalSymbol {
    pub idx: usize,
    pub args: GlobalMacroArgs,
    pub span: Span,
}

#[derive(Copy, Clone, Debug)]
pub enum SymbolType {
    Component(ComponentSymbol),
    Global(GlobalSymbol),
    Trait(GlobalSymbol),
    Event(usize),
    State(usize),
    System(usize, Span),
    ComponentSet(usize),
    Hardcoded(HardcodedSymbol),
}

impl std::fmt::Display for SymbolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SymbolType::Component { .. } => "Component",
            SymbolType::Global { .. } => "Global",
            SymbolType::Trait { .. } => "Trait",
            SymbolType::Event(..) => "Event",
            SymbolType::State(..) => "State",
            SymbolType::System(..) => "System",
            SymbolType::ComponentSet(..) => "ComponentSet",
            SymbolType::Hardcoded(..) => "Hardcoded Path",
        })
    }
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub kind: SymbolType,
    pub path: Vec<String>,
    pub public: bool,
}

// Need &dyn for type alias and passing 'None'
type Location<'a> = (&'a AstMod, &'a dyn Spanned);

impl Symbol {
    fn error(&self, expected: impl Display) -> Error {
        format!("Expected {} but found {}", expected, self.kind).error()
    }
}

pub trait MatchSymbol<'a>
where
    Self: Sized,
{
    fn and_then_impl<T>(self, f: impl FnOnce(&'a Symbol) -> CriticalResult<T>)
        -> CriticalResult<T>;

    fn expect_component(self) -> CriticalResult<(&'a Symbol, ComponentSymbol)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Component(c_sym) => Ok((arg, c_sym)),
            _ => arg.error("Component").as_err(),
        })
    }

    fn expect_global(self) -> CriticalResult<(&'a Symbol, GlobalSymbol)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Global(g_sym) => Ok((arg, g_sym)),
            _ => arg.error("Global").as_err(),
        })
    }

    fn expect_trait(self) -> CriticalResult<(&'a Symbol, GlobalSymbol)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Trait(g_sym) => Ok((arg, g_sym)),
            _ => arg.error("Trait").as_err(),
        })
    }

    fn expect_global_or_trait(self) -> CriticalResult<(&'a Symbol, GlobalSymbol)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Global(g_sym) | SymbolType::Trait(g_sym) => Ok((arg, g_sym)),
            _ => arg.error("Global or Trait").as_err(),
        })
    }

    fn expect_event(self) -> CriticalResult<(&'a Symbol, usize)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Event(i) => Ok((arg, i)),
            _ => arg.error("Event").as_err(),
        })
    }

    fn expect_state(self) -> CriticalResult<(&'a Symbol, usize)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::State(i) => Ok((arg, i)),
            _ => arg.error("State").as_err(),
        })
    }

    fn expect_system(self) -> CriticalResult<(&'a Symbol, (usize, Span))> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::System(i, span) => Ok((arg, (i, span))),
            _ => arg.error("System").as_err(),
        })
    }

    fn expect_component_set(self) -> CriticalResult<(&'a Symbol, usize)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::ComponentSet(i) => Ok((arg, i)),
            _ => arg.error("Component Set").as_err(),
        })
    }

    fn expect_any_hardcoded(self) -> CriticalResult<(&'a Symbol, HardcodedSymbol)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Hardcoded(sym) => Ok((arg, sym)),
            _ => arg.error("Hardcoded Symbol").as_err(),
        })
    }

    fn expect_hardcoded(self, sym: HardcodedSymbol) -> CriticalResult<&'a Symbol> {
        self.expect_any_hardcoded()
            .and_then(|(s, h_sym)| match h_sym == sym {
                true => Ok(s),
                false => s.error(format!("Hardcoded Symbol: '{h_sym:#?}'")).as_err(),
            })
    }
}

// Helper function to just get the data from a resolved symbol
pub trait DiscardSymbol<T> {
    fn discard_symbol(self) -> CriticalResult<T>;
}

impl<T> DiscardSymbol<T> for CriticalResult<(&Symbol, T)> {
    fn discard_symbol(self) -> CriticalResult<T> {
        self.map(|(_, t)| t)
    }
}
