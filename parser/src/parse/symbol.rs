use proc_macro2::Span;
use shared::{
    macros::{expand_enum, ExpandEnum},
    parsing::{ComponentMacroArgs, GlobalMacroArgs},
};
use syn::spanned::Spanned;

use crate::utils::{
    paths::{CratePath, ENGINE_PATHS, MACRO_PATHS},
    Msg, MsgResult,
};

use super::AstMod;

// Symbol with path - Edit this to add new engine items
#[derive(Eq, PartialEq)]
#[expand_enum]
pub enum HardcodedSymbol {
    // Macros crate
    ComponentMacro,
    GlobalMacro,
    EventMacro,
    SystemMacro,
    // Engine crate
    ComponentsMacro,
    Entities,
}

impl HardcodedSymbol {
    pub fn get_path(&self) -> &CratePath {
        match self {
            HardcodedSymbol::ComponentMacro => &MACRO_PATHS.component,
            HardcodedSymbol::GlobalMacro => &MACRO_PATHS.global,
            HardcodedSymbol::EventMacro => &MACRO_PATHS.event,
            HardcodedSymbol::SystemMacro => &MACRO_PATHS.system,
            HardcodedSymbol::ComponentsMacro => &MACRO_PATHS.components,
            HardcodedSymbol::Entities => &ENGINE_PATHS.entities,
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
    fn error_msg(&self, expected: &str, location: Option<Location>) -> Msg {
        let msg = format!("Expected {} but found {}", expected, self.kind);
        match location {
            Some((m, span)) => Msg::for_mod(&msg, m, span),
            None => Msg::String(msg),
        }
    }
}

// Base for match symbol
pub trait MatchSymbolTrait<'a> {
    fn and_then_symbol<T>(self, f: impl FnOnce(&'a Symbol) -> MsgResult<T>) -> MsgResult<T>;

    fn and_then_symbol_in_mod<T>(
        self,
        f: impl FnOnce(&'a Symbol) -> MsgResult<T>,
        m: &AstMod,
        span: &dyn Spanned,
    ) -> MsgResult<T>;
}

// Private part of MatchSymbol
mod match_symbol {
    use super::*;

    pub trait MatchSymbolPrivate<'a>
    where
        Self: Sized + MatchSymbolTrait<'a>,
    {
        fn and_then_impl<T>(
            self,
            f: impl FnOnce(&'a Symbol) -> MsgResult<T>,
            l: Option<Location>,
        ) -> MsgResult<T> {
            match l {
                Some((m, span)) => self.and_then_symbol_in_mod(f, m, span),
                None => self.and_then_symbol(f),
            }
        }

        fn expect_component_impl(
            self,
            l: Option<Location>,
        ) -> MsgResult<(&'a Symbol, ComponentSymbol)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::Component(c_sym) => Ok((arg, c_sym)),
                    _ => Err(vec![arg.error_msg("Component", l)]),
                },
                l,
            )
        }

        fn expect_global_impl(self, l: Option<Location>) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::Global(g_sym) => Ok((arg, g_sym)),
                    _ => Err(vec![arg.error_msg("Global", l)]),
                },
                l,
            )
        }

        fn expect_trait_impl(self, l: Option<Location>) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::Trait(g_sym) => Ok((arg, g_sym)),
                    _ => Err(vec![arg.error_msg("Trait", l)]),
                },
                l,
            )
        }

        fn expect_global_or_trait_impl(
            self,
            l: Option<Location>,
        ) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::Global(g_sym) | SymbolType::Trait(g_sym) => Ok((arg, g_sym)),
                    _ => Err(vec![arg.error_msg("Global or Trait", l)]),
                },
                l,
            )
        }

        fn expect_event_impl(self, l: Option<Location>) -> MsgResult<(&'a Symbol, usize)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::Event(i) => Ok((arg, i)),
                    _ => Err(vec![arg.error_msg("Event", l)]),
                },
                l,
            )
        }

        fn expect_system_impl(self, l: Option<Location>) -> MsgResult<(&'a Symbol, (usize, Span))> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::System(i, s) => Ok((arg, (i, s))),
                    _ => Err(vec![arg.error_msg("System", l)]),
                },
                l,
            )
        }

        fn expect_component_set_impl(self, l: Option<Location>) -> MsgResult<(&'a Symbol, usize)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::ComponentSet(i) => Ok((arg, i)),
                    _ => Err(vec![arg.error_msg("Component Set", l)]),
                },
                l,
            )
        }

        fn expect_any_hardcoded_impl(
            self,
            l: Option<Location>,
        ) -> MsgResult<(&'a Symbol, HardcodedSymbol)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::Hardcoded(sym) => Ok((arg, sym)),
                    _ => Err(vec![arg.error_msg("Hardcoded Symbol", l)]),
                },
                l,
            )
        }

        fn expect_hardcoded_impl(
            self,
            sym: HardcodedSymbol,
            l: Option<Location>,
        ) -> MsgResult<&'a Symbol> {
            self.expect_any_hardcoded_impl(l)
                .and_then(|(s, h_sym)| match h_sym == sym {
                    true => Ok(s),
                    false => Err(vec![
                        s.error_msg(&format!("Hardcoded Symbol: '{h_sym:#?}'"), l)
                    ]),
                })
        }
    }

    impl<'a, T> MatchSymbolPrivate<'a> for T where T: MatchSymbolTrait<'a> {}
}

// Public part of MatchSymbol
pub trait MatchSymbol<'a>
where
    Self: Sized + match_symbol::MatchSymbolPrivate<'a>,
{
    fn expect_component(self) -> MsgResult<(&'a Symbol, ComponentSymbol)> {
        self.expect_component_impl(None)
    }

    fn expect_component_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> MsgResult<(&'a Symbol, ComponentSymbol)> {
        self.expect_component_impl(Some((m, span)))
    }

    fn expect_global(self) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_global_impl(None)
    }

    fn expect_global_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_global_impl(Some((m, span)))
    }

    fn expect_trait(self) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_trait_impl(None)
    }

    fn expect_trait_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_trait_impl(Some((m, span)))
    }

    fn expect_global_or_trait(self) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_global_or_trait_impl(None)
    }

    fn expect_global_or_trait_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_global_or_trait_impl(Some((m, span)))
    }

    fn expect_event(self) -> MsgResult<(&'a Symbol, usize)> {
        self.expect_event_impl(None)
    }

    fn expect_event_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> MsgResult<(&'a Symbol, usize)> {
        self.expect_event_impl(Some((m, span)))
    }

    fn expect_system(self) -> MsgResult<(&'a Symbol, (usize, Span))> {
        self.expect_system_impl(None)
    }

    fn expect_system_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> MsgResult<(&'a Symbol, (usize, Span))> {
        self.expect_system_impl(Some((m, span)))
    }

    fn expect_component_set(self) -> MsgResult<(&'a Symbol, usize)> {
        self.expect_component_set_impl(None)
    }

    fn expect_component_set_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> MsgResult<(&'a Symbol, usize)> {
        self.expect_component_set_impl(Some((m, span)))
    }

    fn expect_any_hardcoded(self) -> MsgResult<(&'a Symbol, HardcodedSymbol)> {
        self.expect_any_hardcoded_impl(None)
    }

    fn expect_any_hardcoded_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> MsgResult<(&'a Symbol, HardcodedSymbol)> {
        self.expect_any_hardcoded_impl(Some((m, span)))
    }

    fn expect_hardcoded(self, sym: HardcodedSymbol) -> MsgResult<&'a Symbol> {
        self.expect_hardcoded_impl(sym, None)
    }

    fn expect_hardcoded_in_mod(
        self,
        sym: HardcodedSymbol,
        m: &AstMod,
        span: &impl Spanned,
    ) -> MsgResult<&'a Symbol> {
        self.expect_hardcoded_impl(sym, Some((m, span)))
    }
}

impl<'a, T> MatchSymbol<'a> for T where T: match_symbol::MatchSymbolPrivate<'a> {}

impl<'a> MatchSymbolTrait<'a> for MsgResult<&'a Symbol> {
    fn and_then_symbol<T>(self, f: impl FnOnce(&'a Symbol) -> MsgResult<T>) -> MsgResult<T> {
        self.and_then(f)
    }

    fn and_then_symbol_in_mod<T>(
        self,
        f: impl FnOnce(&'a Symbol) -> MsgResult<T>,
        m: &AstMod,
        span: &dyn Spanned,
    ) -> MsgResult<T> {
        self.and_then(f)
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
