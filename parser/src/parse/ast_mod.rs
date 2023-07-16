use std::{
    fs,
    path::{Display, PathBuf},
};

use proc_macro2::{Span, TokenStream};

use syn::{spanned::Spanned, visit::Visit};

use crate::{
    parse::attributes::{get_attributes_if_active, Attribute, EcsAttribute},
    parse::ItemPath,
    utils::{
        paths::{CratePath, ENGINE_PATHS, MACRO_PATHS},
        syn::{add_use_item, use_path_from_syn, ToRange},
        Msg, MsgResult, SpanFiles,
    },
};

use super::{attributes::AstAttribute, Symbol};

use shared::{
    macros::{expand_enum, ExpandEnum},
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
    pub span: Span,
}

#[derive(Debug)]
pub struct AstItem<Data> {
    pub data: Data,
    pub path: Vec<String>,
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
struct VisitorArgs<'a> {
    span_files: &'a mut SpanFiles,
}

#[derive(Debug)]
pub struct AstMod {
    pub ty: AstModType,
    pub dir: PathBuf,
    pub span_file: usize,
    pub span_start: Option<usize>,
    pub path: Vec<String>,
    pub mods: Vec<AstMod>,
    pub uses: Vec<AstUse>,
    pub symbols: Vec<Symbol>,
    pub items: AstItems,
}

// TODO: ignore private mods to avoid name collisions
// Pass 1: parsing
impl AstMod {
    pub fn new(
        dir: PathBuf,
        path: Vec<String>,
        ty: AstModType,
        span_file: usize,
        span_start: Option<usize>,
    ) -> Self {
        Self {
            ty,
            dir,
            span_file,
            span_start,
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

    pub fn parse(
        span_files: &mut SpanFiles,
        dir: PathBuf,
        file: PathBuf,
        path: Vec<String>,
        ty: AstModType,
    ) -> Self {
        let file_contents = fs::read_to_string(file.to_owned())
            .catch(format!("Failed to read file: {}", file.display()));
        let ast = syn::parse_file(&file_contents).catch(format!(
            "Failed to parse file contents of: {}",
            file.display()
        ));

        let mut s = Self::new(
            dir,
            path,
            ty,
            span_files.add(file.display().to_string(), file_contents),
            ast.span().to_range().ok().map(|r| r.start),
        );

        s.visit_file(ast, &mut VisitorArgs { span_files });
        // Post processing
        s.resolve_local_use_paths();

        s
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
    fn visit_file(&mut self, i: syn::File, args: &mut VisitorArgs) {
        self.visit_items(i.items, args);
    }

    fn visit_items(&mut self, items: Vec<syn::Item>, args: &mut VisitorArgs) {
        for item in items {
            self.visit_item(item, args);
        }
    }

    fn visit_item(&mut self, i: syn::Item, args: &mut VisitorArgs) {
        // Match again to parse
        match i {
            syn::Item::Mod(i) => self.visit_item_mod(i, args),
            syn::Item::Use(i) => self.visit_item_use(i),
            syn::Item::Fn(i) => self.visit_item_fn(i),
            syn::Item::Enum(i) => self.visit_item_enum(i),
            syn::Item::Struct(i) => self.visit_item_struct(i),
            syn::Item::Macro(i) => self.visit_item_macro(i),
            _ => (),
        }
    }

    // Mod
    fn visit_item_mod(&mut self, i: syn::ItemMod, args: &mut VisitorArgs) {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new()) {
            self.mods.push(match i.content {
                // Parse inner mod
                Some((_, items)) => {
                    let mut new_mod = Self::new(
                        self.dir.to_owned(),
                        [self.path.to_vec(), vec![i.ident.to_string()]].concat(),
                        AstModType::Internal,
                        self.span_file,
                        self.span_start,
                    );
                    new_mod.visit_items(items, args);
                    new_mod
                }
                // Parse file mod
                None => Self::parse_mod(
                    args.span_files,
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
                    data: AstMacroCall {
                        span: i.mac.span(),
                        args: i.mac.tokens,
                    },
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
