use std::{
    fs,
    path::{Display, PathBuf},
};

use diagnostic::{ok, CatchErr, ErrForEach, ErrorSpan, ErrorTrait, ResultsTrait, ToErr};
use proc_macro2::{Span, TokenStream};

use syn::{spanned::Spanned, visit::Visit};

use crate::{
    parse::attributes::{get_attributes_if_active, Attribute, EcsAttribute},
    parse::ItemPath,
    utils::{
        paths::{CratePath, ENGINE_PATHS, MACRO_PATHS},
        tree::Tree,
    },
};

use super::{
    attributes::AstAttribute,
    items::{AstItems, AstUse},
    AstItemData, Symbol,
};

use shared::{
    macros::{expand_enum, ExpandEnum},
    parsing::{ComponentMacroArgs, GlobalMacroArgs},
    syn::{
        add_use_item,
        error::{CriticalResult, Error, MutateResults, Result, StrToError, ToError, WarningResult},
        use_path_from_syn, ToRange,
    },
    traits::{Call, Catch, CollectVecInto, HandleErr, PushInto},
};

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
pub struct NewMod {
    pub name: String,
    pub mods: Vec<NewMod>,
    pub uses: Vec<AstUse>,
    pub symbols: Vec<Symbol>,
    pub span: Span,
}

#[derive(Debug)]
pub struct AstMod {
    pub idx: usize,
    pub ty: AstModType,
    pub file: PathBuf,
    pub path: Vec<String>,
    pub name: String,
    pub span: Span,
    pub mods: Vec<usize>,
    pub uses: Vec<AstUse>,
    pub symbols: Vec<Symbol>,
    pub items: AstItems,
}

pub type AstModTree = Tree<WarningResult<AstMod>>;

// TODO: ignore private mods to avoid name collisions
// Pass 1: parsing
impl AstMod {
    pub fn new(
        idx: usize,
        file: PathBuf,
        path: Vec<String>,
        name: String,
        ty: AstModType,
        span: Span,
    ) -> Self {
        Self {
            idx,
            ty,
            file,
            name,
            path,
            span,
            mods: Vec::new(),
            uses: Vec::new(),
            symbols: Vec::new(),
            items: AstItems::new(),
        }
    }

    pub fn parse(file: PathBuf, path: Vec<String>, ty: AstModType) -> CriticalResult<AstModTree> {
        let file_contents = fs::read_to_string(file.to_owned())
            .catch_err(format!("Failed to read file: {}", file.display()).trace())?;
        let ast = syn::parse_file(&file_contents)
            .catch_err(format!("Failed to parse file contents of: {}", file.display()).trace())?;
        let name = path
            .last()
            .catch_err(format!("Mod path is empty: {}", path.join("::")).trace())?
            .to_string();

        let mut root = Self::new(0, file, path, name, ty, ast.span());

        let mods = root.visit_items(ast.items);
        // Resolve local use paths
        // E.g. mod Foo; use Foo::Bar;
        for use_path in root.uses.iter_mut() {
            if let Some(m) = mods
                .value
                .iter()
                .find(|m| m.root.path.last().is_some_and(|p| p == &use_path.first))
            {
                use_path.path = [m.root.path.to_vec(), use_path.path[1..].to_vec()].concat();
            }
        }

        Ok(mods.map(|children| Tree {
            root: ok(root),
            children,
        }))
    }

    pub fn add_symbol(&mut self, symbol: Symbol) {
        self.symbols.push(symbol)
    }

    pub fn get_file(&self) -> String {
        self.file.display().to_string()
    }

    // Add new mods to the given mods and return all new mods in lrn order
    pub fn add_mod(&mut self, mut num_mods: usize, data: NewMod) -> Vec<AstMod> {
        let mut new_mod = AstMod::new(
            0,
            self.file.clone(),
            self.path.to_vec().push_into(data.name.to_string()),
            data.name,
            AstModType::Internal,
            data.span,
        );
        new_mod.symbols = data.symbols;
        new_mod.uses = data.uses;
        let mut mods = Vec::new();
        for m in data.mods {
            let child_mods = new_mod.add_mod(num_mods, m);
            num_mods += child_mods.len();
            mods.extend(child_mods);
        }
        new_mod.idx = num_mods;
        self.mods.push(num_mods);
        mods.push_into(new_mod)
    }
}

// File/items
impl AstMod {
    fn visit_items(&mut self, items: Vec<syn::Item>) -> Vec<AstModTree> {
        let mut mods = Vec::new();
        items
            .try_for_each(|i| match i {
                syn::Item::Use(i) => self.visit_item_use(i),
                syn::Item::Fn(i) => self.visit_item_fn(i),
                syn::Item::Enum(i) => self.visit_item_enum(i),
                syn::Item::Struct(i) => self.visit_item_struct(i),
                syn::Item::Macro(i) => self.visit_item_macro(i),
                syn::Item::Mod(i) => self.visit_item_mod(i).map(|tree| mods.push(tree)),
                _ => Ok(()),
            })
            .map(|_| mods)
    }

    // Mod
    fn visit_item_mod(&mut self, i: syn::ItemMod) -> CriticalResult<AstModTree> {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new())? {
            match i.content {
                // Parse inner mod
                Some((_, items)) => {
                    let mut new_mod = Self::new(
                        0,
                        self.file.to_owned(),
                        [self.path.to_vec(), vec![i.ident.to_string()]].concat(),
                        i.ident.to_string(),
                        AstModType::Internal,
                        self.span,
                    );
                    Ok(Tree {
                        children: new_mod.visit_items(items),
                        root: ok(new_mod),
                    })
                }
                // Parse file mod
                None => Self::parse_mod(
                    self.file
                        .to_owned()
                        .pop_into()
                        .push_into(i.ident.to_string()),
                    &[self.path.to_vec(), vec![i.ident.to_string()]].concat(),
                )
                .with_span(&i.ident),
            }
        } else {
            Err(Vec::new())
        }
    }
}
