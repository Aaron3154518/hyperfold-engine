use std::{fs, path::PathBuf};

use proc_macro2::TokenStream;

use syn::visit::Visit;

use crate::{
    parse::attributes::{get_attributes_if_active, Attribute, EcsAttribute},
    parse::ItemPath,
    utils::{
        paths::{CratePath, ENGINE_PATHS, MACRO_PATHS},
        syn::{add_use_item, use_path_from_syn},
    },
};

use super::attributes::AstAttribute;

use shared::{
    macros::{expand_enum, ExpandEnum},
    msg_result::MsgResult,
    parsing::{ComponentMacroArgs, GlobalMacroArgs},
    traits::{Catch, PushInto},
};

#[derive(Debug)]
pub struct AstStruct {
    pub attrs: Vec<AstAttribute>,
}

#[derive(Debug)]
pub struct AstEnum {
    pub attrs: Vec<AstAttribute>,
}

#[derive(Debug)]
pub struct AstFunction {
    pub sig: syn::Signature,
    pub attrs: Vec<AstAttribute>,
}

#[derive(Debug)]
pub struct AstMacroCall {
    pub args: TokenStream,
}

#[derive(Debug)]
pub struct AstItem<Data> {
    pub data: Data,
    pub path: Vec<String>,
}

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
    System(usize),
    ComponentSet(usize),
    Hardcoded(HardcodedSymbol),
}

impl std::fmt::Display for SymbolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SymbolType::Component { .. } => "Component",
            SymbolType::Global { .. } => "Global",
            SymbolType::Trait { .. } => "Trait",
            SymbolType::Event(_) => "Event",
            SymbolType::System(_) => "System",
            SymbolType::ComponentSet(_) => "ComponentSet",
            SymbolType::Hardcoded(_) => "Hardcoded Path",
        })
    }
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub kind: SymbolType,
    pub path: Vec<String>,
    pub public: bool,
}

impl Symbol {
    pub fn from(
        kind: SymbolType,
        mut path: Vec<String>,
        ident: &syn::Ident,
        vis: &syn::Visibility,
    ) -> Self {
        path.push(ident.to_string());
        // Add to the symbol table
        Self {
            kind,
            path,
            public: match vis {
                syn::Visibility::Public(_) => true,
                syn::Visibility::Restricted(_) | syn::Visibility::Inherited => false,
            },
        }
    }

    fn panic_msg(&self, expected: &str) -> String {
        format!(
            "When resolving '{}': Expected '{}' but found '{}'",
            self.path.join("::"),
            expected,
            self.kind
        )
    }
}

pub trait MatchSymbol<'a> {
    fn expect_component(self) -> MsgResult<(&'a Symbol, ComponentSymbol)>;

    fn expect_global(self) -> MsgResult<(&'a Symbol, GlobalSymbol)>;

    fn expect_trait(self) -> MsgResult<(&'a Symbol, GlobalSymbol)>;

    fn expect_global_or_trait(self) -> MsgResult<(&'a Symbol, GlobalSymbol)>;

    fn expect_event(self) -> MsgResult<(&'a Symbol, usize)>;

    fn expect_system(self) -> MsgResult<(&'a Symbol, usize)>;

    fn expect_component_set(self) -> MsgResult<(&'a Symbol, usize)>;

    fn expect_any_hardcoded(self) -> MsgResult<(&'a Symbol, HardcodedSymbol)>;

    fn expect_hardcoded(self, sym: HardcodedSymbol) -> MsgResult<&'a Symbol>;
}

impl<'a> MatchSymbol<'a> for MsgResult<&'a Symbol> {
    fn expect_component(self) -> MsgResult<(&'a Symbol, ComponentSymbol)> {
        self.and_then(|arg| match arg.kind {
            SymbolType::Component(c_sym) => Ok((arg, c_sym)),
            _ => Err(vec![arg.panic_msg("Component")]),
        })
    }

    fn expect_global(self) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.and_then(|arg| match arg.kind {
            SymbolType::Global(g_sym) => Ok((arg, g_sym)),
            _ => Err(vec![arg.panic_msg("Global")]),
        })
    }

    fn expect_trait(self) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.and_then(|arg| match arg.kind {
            SymbolType::Trait(g_sym) => Ok((arg, g_sym)),
            _ => Err(vec![arg.panic_msg("Trait")]),
        })
    }

    fn expect_global_or_trait(self) -> MsgResult<(&'a Symbol, GlobalSymbol)> {
        self.and_then(|arg| match arg.kind {
            SymbolType::Global(g_sym) | SymbolType::Trait(g_sym) => Ok((arg, g_sym)),
            _ => Err(vec![arg.panic_msg("Trait or Global")]),
        })
    }

    fn expect_event(self) -> MsgResult<(&'a Symbol, usize)> {
        self.and_then(|arg| match arg.kind {
            SymbolType::Event(i) => Ok((arg, i)),
            _ => Err(vec![arg.panic_msg("Event")]),
        })
    }

    fn expect_system(self) -> MsgResult<(&'a Symbol, usize)> {
        self.and_then(|arg| match arg.kind {
            SymbolType::System(i) => Ok((arg, i)),
            _ => Err(vec![arg.panic_msg("System")]),
        })
    }

    fn expect_component_set(self) -> MsgResult<(&'a Symbol, usize)> {
        self.and_then(|arg| match arg.kind {
            SymbolType::ComponentSet(i) => Ok((arg, i)),
            _ => Err(vec![arg.panic_msg("Component Set")]),
        })
    }

    fn expect_any_hardcoded(self) -> MsgResult<(&'a Symbol, HardcodedSymbol)> {
        self.and_then(|arg| match arg.kind {
            SymbolType::Hardcoded(sym) => Ok((arg, sym)),
            _ => Err(vec![arg.panic_msg("Hardcoded Path")]),
        })
    }

    fn expect_hardcoded(self, sym: HardcodedSymbol) -> MsgResult<&'a Symbol> {
        self.expect_any_hardcoded()
            .and_then(|(s, h_sym)| match h_sym == sym {
                true => Ok(s),
                false => Err(vec![s.panic_msg(&format!("Hardcoded Path: {s:#?}"))]),
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

// Use statement
#[derive(Clone, Debug)]
pub struct AstUse {
    pub ident: String,
    pub path: Vec<String>,
    pub public: bool,
}

impl AstUse {
    pub fn from(mut path: Vec<String>, ident: &syn::Ident, vis: &syn::Visibility) -> Self {
        path.push(ident.to_string());
        Self {
            ident: ident.to_string(),
            path,
            public: match vis {
                syn::Visibility::Public(_) => true,
                syn::Visibility::Restricted(_) | syn::Visibility::Inherited => false,
            },
        }
    }
}

#[derive(Debug)]
pub struct AstItems {
    pub structs: Vec<AstItem<AstStruct>>,
    pub enums: Vec<AstItem<AstEnum>>,
    pub functions: Vec<AstItem<AstFunction>>,
    pub macro_calls: Vec<AstItem<AstMacroCall>>,
}

// Type of module
#[derive(Debug)]
pub enum AstModType {
    Main,
    Lib,
    Mod,
    File,
    Internal,
}

// Module
#[derive(Debug)]
pub struct AstMod {
    pub ty: AstModType,
    pub dir: PathBuf,
    pub path: Vec<String>,
    pub mods: Vec<AstMod>,
    pub uses: Vec<AstUse>,
    pub symbols: Vec<Symbol>,
    pub items: AstItems,
}

// TODO: ignore private mods to avoid name collisions
// Pass 1: parsing
impl AstMod {
    pub fn new(dir: PathBuf, path: Vec<String>, ty: AstModType) -> Self {
        Self {
            ty,
            dir,
            path,
            mods: Vec::new(),
            uses: Vec::new(),
            symbols: Vec::new(),
            items: AstItems {
                structs: Vec::new(),
                enums: Vec::new(),
                functions: Vec::new(),
                macro_calls: Vec::new(),
            },
        }
    }

    pub fn parse(&mut self, path: PathBuf) {
        let file_contents = fs::read_to_string(path.to_owned())
            .catch(format!("Failed to read file: {}", path.display()));
        let ast = syn::parse_file(&file_contents).catch(format!(
            "Failed to parse file contents of: {}",
            path.display()
        ));
        self.visit_file(ast);

        // Post processing
        self.resolve_local_use_paths();
    }

    // E.g. mod Foo; use Foo::Bar;
    pub fn resolve_local_use_paths(&mut self) {
        for use_path in self.uses.iter_mut() {
            let first = use_path.path.first().expect("Empty use path");
            if let Some(m) = self
                .mods
                .iter()
                .find(|m| m.path.last().is_some_and(|p| p == first))
            {
                use_path.path = [m.path.to_vec(), use_path.path[1..].to_vec()].concat();
            }
        }
    }
}

// File/items
impl AstMod {
    pub fn visit_file(&mut self, i: syn::File) {
        self.visit_items(i.items);
    }

    fn visit_items(&mut self, items: Vec<syn::Item>) {
        for item in items {
            self.visit_item(item);
        }
    }

    fn visit_item(&mut self, i: syn::Item) {
        // Match again to parse
        match i {
            syn::Item::Mod(i) => self.visit_item_mod(i),
            syn::Item::Use(i) => self.visit_item_use(i),
            syn::Item::Fn(i) => self.visit_item_fn(i),
            syn::Item::Enum(i) => self.visit_item_enum(i),
            syn::Item::Struct(i) => self.visit_item_struct(i),
            syn::Item::Macro(i) => self.visit_item_macro(i),
            _ => (),
        }
    }

    // Mod
    fn visit_item_mod(&mut self, i: syn::ItemMod) {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new()) {
            self.mods.push(match i.content {
                // Parse inner mod
                Some((_, items)) => {
                    let mut new_mod = Self::new(
                        self.dir.to_owned(),
                        [self.path.to_vec(), vec![i.ident.to_string()]].concat(),
                        AstModType::Internal,
                    );
                    new_mod.visit_items(items);
                    new_mod
                }
                // Parse file mod
                None => Self::parse_mod(
                    self.dir.join(i.ident.to_string()),
                    &[self.path.to_vec(), vec![i.ident.to_string()]].concat(),
                ),
            });
        }
    }

    // Components
    fn visit_item_struct(&mut self, i: syn::ItemStruct) {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new()) {
            if !attrs.is_empty() {
                self.items.structs.push(AstItem {
                    data: AstStruct { attrs },
                    path: self.path.to_vec().push_into(i.ident.to_string()),
                });
            }
        }
    }

    fn visit_item_enum(&mut self, i: syn::ItemEnum) {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new()) {
            if !attrs.is_empty() {
                self.items.enums.push(AstItem {
                    data: AstEnum { attrs },
                    path: self.path.to_vec().push_into(i.ident.to_string()),
                });
            }
        }
    }

    // Systems
    fn visit_item_fn(&mut self, i: syn::ItemFn) {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new()) {
            if !attrs.is_empty() {
                self.items.functions.push(AstItem {
                    path: self.path.to_vec().push_into(i.sig.ident.to_string()),
                    data: AstFunction { sig: i.sig, attrs },
                });
            }
        }
    }

    // Macro call
    fn visit_item_macro(&mut self, i: syn::ItemMacro) {
        if let Some(_) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new()) {
            // Some is for macro_rules!
            if i.ident.is_none() {
                self.items.macro_calls.push(AstItem {
                    path: use_path_from_syn(&self.path, &i.mac.path),
                    data: AstMacroCall { args: i.mac.tokens },
                });
            }
        }
    }
}

// TODO: Ambiguous paths, :: paths
// TODO: type redeclarations

// Use paths
impl AstMod {
    fn visit_item_use(&mut self, i: syn::ItemUse) {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new()) {
            let mut uses = self.visit_use_tree(i.tree, &mut Vec::new(), Vec::new());
            let public = match i.vis {
                syn::Visibility::Public(_) => true,
                syn::Visibility::Restricted(_) | syn::Visibility::Inherited => false,
            };
            uses.iter_mut().for_each(|u| u.public = public);
            self.uses.append(&mut uses);
        }
    }

    fn visit_use_tree(
        &mut self,
        i: syn::UseTree,
        path: &mut Vec<String>,
        items: Vec<AstUse>,
    ) -> Vec<AstUse> {
        match i {
            syn::UseTree::Path(i) => self.visit_use_path(i, path, items),
            syn::UseTree::Name(i) => self.visit_use_name(i, path, items),
            syn::UseTree::Rename(i) => self.visit_use_rename(i, path, items),
            syn::UseTree::Glob(i) => self.visit_use_glob(i, path, items),
            syn::UseTree::Group(i) => self.visit_use_group(i, path, items),
        }
    }

    fn visit_use_path(
        &mut self,
        i: syn::UsePath,
        path: &mut Vec<String>,
        items: Vec<AstUse>,
    ) -> Vec<AstUse> {
        add_use_item(&self.path, path, i.ident.to_string());
        self.visit_use_tree(*i.tree, path, items)
    }

    fn visit_use_name(
        &mut self,
        i: syn::UseName,
        path: &mut Vec<String>,
        mut items: Vec<AstUse>,
    ) -> Vec<AstUse> {
        add_use_item(&self.path, path, i.ident.to_string());
        items.push(AstUse {
            ident: path.last().expect("Empty use path with 'self'").to_string(),
            path: path.to_vec(),
            public: false,
        });
        items
    }

    fn visit_use_rename(
        &mut self,
        i: syn::UseRename,
        path: &mut Vec<String>,
        mut items: Vec<AstUse>,
    ) -> Vec<AstUse> {
        items.push(AstUse {
            ident: i.rename.to_string(),
            path: [path.to_owned(), vec![i.ident.to_string()]].concat(),
            public: false,
        });
        items
    }

    fn visit_use_glob(
        &mut self,
        i: syn::UseGlob,
        path: &mut Vec<String>,
        mut items: Vec<AstUse>,
    ) -> Vec<AstUse> {
        items.push(AstUse {
            ident: "*".to_string(),
            path: path.to_owned(),
            public: false,
        });
        items
    }

    fn visit_use_group(
        &mut self,
        i: syn::UseGroup,
        path: &mut Vec<String>,
        items: Vec<AstUse>,
    ) -> Vec<AstUse> {
        i.items.into_iter().fold(items, |items, i| {
            self.visit_use_tree(i, &mut path.to_vec(), items)
        })
    }
}
