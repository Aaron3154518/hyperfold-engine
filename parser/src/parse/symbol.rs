use proc_macro2::Span;

use syn::spanned::Spanned;

use crate::utils::paths::{CratePath, ENGINE_PATHS, MACRO_PATHS};

use super::AstMod;

use shared::{
    macros::{expand_enum, ExpandEnum},
    parsing::{ComponentMacroArgs, GlobalMacroArgs},
    syn::{DiagnosticResult, Msg},
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
    fn error_msg(&self, expected: &str, location: Option<Location>) -> Msg {
        let msg = format!("Expected {} but found {}", expected, self.kind);
        match location {
            Some((m, span)) => Msg::for_mod(&msg, m, span),
            None => Error::new(&msg),
        }
    }
}

// Base for match symbol
pub trait MatchSymbolTrait<'a> {
    fn and_then_symbol<T>(
        self,
        f: impl FnOnce(&'a Symbol) -> DiagnosticResult<T>,
    ) -> DiagnosticResult<T>;

    fn and_then_symbol_in_mod<T>(
        self,
        f: impl FnOnce(&'a Symbol) -> DiagnosticResult<T>,
        m: &AstMod,
        span: &dyn Spanned,
    ) -> DiagnosticResult<T>;
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
            f: impl FnOnce(&'a Symbol) -> DiagnosticResult<T>,
            l: Option<Location>,
        ) -> DiagnosticResult<T> {
            match l {
                Some((m, span)) => self.and_then_symbol_in_mod(f, m, span),
                None => self.and_then_symbol(f),
            }
        }

        fn expect_component_impl(
            self,
            l: Option<Location>,
        ) -> DiagnosticResult<(&'a Symbol, ComponentSymbol)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::Component(c_sym) => Ok((arg, c_sym)),
                    _ => Err(vec![arg.error_msg("Component", l)]),
                },
                l,
            )
        }

        fn expect_global_impl(
            self,
            l: Option<Location>,
        ) -> DiagnosticResult<(&'a Symbol, GlobalSymbol)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::Global(g_sym) => Ok((arg, g_sym)),
                    _ => Err(vec![arg.error_msg("Global", l)]),
                },
                l,
            )
        }

        fn expect_trait_impl(
            self,
            l: Option<Location>,
        ) -> DiagnosticResult<(&'a Symbol, GlobalSymbol)> {
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
        ) -> DiagnosticResult<(&'a Symbol, GlobalSymbol)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::Global(g_sym) | SymbolType::Trait(g_sym) => Ok((arg, g_sym)),
                    _ => Err(vec![arg.error_msg("Global or Trait", l)]),
                },
                l,
            )
        }

        fn expect_event_impl(self, l: Option<Location>) -> DiagnosticResult<(&'a Symbol, usize)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::Event(i) => Ok((arg, i)),
                    _ => Err(vec![arg.error_msg("Event", l)]),
                },
                l,
            )
        }

        fn expect_state_impl(self, l: Option<Location>) -> DiagnosticResult<(&'a Symbol, usize)> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::State(i) => Ok((arg, i)),
                    _ => Err(vec![arg.error_msg("State", l)]),
                },
                l,
            )
        }

        fn expect_system_impl(
            self,
            l: Option<Location>,
        ) -> DiagnosticResult<(&'a Symbol, (usize, Span))> {
            self.and_then_impl(
                |arg| match arg.kind {
                    SymbolType::System(i, s) => Ok((arg, (i, s))),
                    _ => Err(vec![arg.error_msg("System", l)]),
                },
                l,
            )
        }

        fn expect_component_set_impl(
            self,
            l: Option<Location>,
        ) -> DiagnosticResult<(&'a Symbol, usize)> {
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
        ) -> DiagnosticResult<(&'a Symbol, HardcodedSymbol)> {
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
        ) -> DiagnosticResult<&'a Symbol> {
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
    fn expect_component(self) -> DiagnosticResult<(&'a Symbol, ComponentSymbol)> {
        self.expect_component_impl(None)
    }

    fn expect_component_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> DiagnosticResult<(&'a Symbol, ComponentSymbol)> {
        self.expect_component_impl(Some((m, span)))
    }

    fn expect_global(self) -> DiagnosticResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_global_impl(None)
    }

    fn expect_global_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> DiagnosticResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_global_impl(Some((m, span)))
    }

    fn expect_trait(self) -> DiagnosticResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_trait_impl(None)
    }

    fn expect_trait_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> DiagnosticResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_trait_impl(Some((m, span)))
    }

    fn expect_global_or_trait(self) -> DiagnosticResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_global_or_trait_impl(None)
    }

    fn expect_global_or_trait_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> DiagnosticResult<(&'a Symbol, GlobalSymbol)> {
        self.expect_global_or_trait_impl(Some((m, span)))
    }

    fn expect_event(self) -> DiagnosticResult<(&'a Symbol, usize)> {
        self.expect_event_impl(None)
    }

    fn expect_event_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> DiagnosticResult<(&'a Symbol, usize)> {
        self.expect_event_impl(Some((m, span)))
    }

    fn expect_state(self) -> DiagnosticResult<(&'a Symbol, usize)> {
        self.expect_state_impl(None)
    }

    fn expect_state_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> DiagnosticResult<(&'a Symbol, usize)> {
        self.expect_state_impl(Some((m, span)))
    }

    fn expect_system(self) -> DiagnosticResult<(&'a Symbol, (usize, Span))> {
        self.expect_system_impl(None)
    }

    fn expect_system_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> DiagnosticResult<(&'a Symbol, (usize, Span))> {
        self.expect_system_impl(Some((m, span)))
    }

    fn expect_component_set(self) -> DiagnosticResult<(&'a Symbol, usize)> {
        self.expect_component_set_impl(None)
    }

    fn expect_component_set_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> DiagnosticResult<(&'a Symbol, usize)> {
        self.expect_component_set_impl(Some((m, span)))
    }

    fn expect_any_hardcoded(self) -> DiagnosticResult<(&'a Symbol, HardcodedSymbol)> {
        self.expect_any_hardcoded_impl(None)
    }

    fn expect_any_hardcoded_in_mod(
        self,
        m: &AstMod,
        span: &impl Spanned,
    ) -> DiagnosticResult<(&'a Symbol, HardcodedSymbol)> {
        self.expect_any_hardcoded_impl(Some((m, span)))
    }

    fn expect_hardcoded(self, sym: HardcodedSymbol) -> DiagnosticResult<&'a Symbol> {
        self.expect_hardcoded_impl(sym, None)
    }

    fn expect_hardcoded_in_mod(
        self,
        sym: HardcodedSymbol,
        m: &AstMod,
        span: &impl Spanned,
    ) -> DiagnosticResult<&'a Symbol> {
        self.expect_hardcoded_impl(sym, Some((m, span)))
    }
}

impl<'a, T> MatchSymbol<'a> for T where T: match_symbol::MatchSymbolPrivate<'a> {}

// Helper function to just get the data from a resolved symbol
pub trait DiscardSymbol<T> {
    fn discard_symbol(self) -> DiagnosticResult<T>;
}

impl<T> DiscardSymbol<T> for DiagnosticResult<(&Symbol, T)> {
    fn discard_symbol(self) -> DiagnosticResult<T> {
        self.map(|(_, t)| t)
    }
}
