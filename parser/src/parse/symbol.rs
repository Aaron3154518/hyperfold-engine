use diagnostic::{Results, ToErr};
use proc_macro2::Span;

use syn::spanned::Spanned;

use crate::utils::paths::{CratePath, ENGINE_PATHS, MACRO_PATHS};

use super::AstMod;

use shared::{
    macros::{expand_enum, ExpandEnum},
    parsing::{ComponentMacroArgs, GlobalMacroArgs},
    syn::error::MsgResult,
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

#[derive(PartialEq, Eq, Copy, Clone, Hash, Debug)]
pub struct ComponentSymbol {
    pub idx: usize,
    pub args: ComponentMacroArgs,
}

#[derive(Copy, Clone, Debug)]
pub struct GlobalSymbol {
    pub idx: usize,
    pub args: GlobalMacroArgs,
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
    fn error(&self, expected: &str) -> String {
        format!("Expected {} but found {}", expected, self.kind)
    }
}

pub trait MatchSymbol<'a>
where
    Self: Sized,
{
    fn and_then_impl<T>(self, f: impl FnOnce(&'a Symbol) -> MsgResult<T>) -> MsgResult<T>;

    fn expect_component(self) -> MsgResult<(&'a Symbol, ComponentSymbol)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Component(c_sym) => Ok((arg, c_sym)),
            _ => arg.error("Component").err(),
        })
    }

    fn expect_global(self) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Global(g_sym) => Ok((arg, g_sym)),
            _ => arg.error("Global").err(),
        })
    }

    fn expect_trait(self) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Trait(g_sym) => Ok((arg, g_sym)),
            _ => arg.error("Trait").err(),
        })
    }

    fn expect_global_or_trait(self) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Global(g_sym) | SymbolType::Trait(g_sym) => Ok((arg, g_sym)),
            _ => arg.error("Global or Trait").err(),
        })
    }

    fn expect_event(self) -> MsgResult<(&'a Symbol, usize)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Event(i) => Ok((arg, i)),
            _ => arg.error("Event").err(),
        })
    }

    fn expect_state(self) -> MsgResult<(&'a Symbol, usize)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::State(i) => Ok((arg, i)),
            _ => arg.error("State").err(),
        })
    }

    fn expect_system(self) -> MsgResult<(&'a Symbol, (usize, Span))> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::System(i, span) => Ok((arg, (i, span))),
            _ => arg.error("System").err(),
        })
    }

    fn expect_component_set(self) -> MsgResult<(&'a Symbol, usize)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::ComponentSet(i) => Ok((arg, i)),
            _ => arg.error("Component Set").err(),
        })
    }

    fn expect_any_hardcoded(self) -> MsgResult<(&'a Symbol, HardcodedSymbol)> {
        self.and_then_impl(|arg| match arg.kind {
            SymbolType::Hardcoded(sym) => Ok((arg, sym)),
            _ => arg.error("Hardcoded Symbol").err(),
        })
    }

    fn expect_hardcoded(self, sym: HardcodedSymbol) -> MsgResult<&'a Symbol> {
        self.expect_any_hardcoded()
            .and_then(|(s, h_sym)| match h_sym == sym {
                true => Ok(s),
                false => s.error(&format!("Hardcoded Symbol: '{h_sym:#?}'")).err(),
            })
    }
}

// Helper function to just get the data from a resolved symbol
pub trait DiscardSymbol<T> {
    fn discard_symbol(self) -> MsgResult<T>;
}

impl<T> DiscardSymbol<T> for MsgResult<(&Symbol, T)> {
    fn discard_symbol(self) -> MsgResult<T> {
        self.map(|(_, t)| t)
    }
}
