use std::{
    fs,
    path::{Display, PathBuf},
};

use diagnostic::{CatchErr, ErrForEach, ErrorSpan, ResultsTrait, ToErr};
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
    msg_result::ToMsgs,
    parsing::{ComponentMacroArgs, GlobalMacroArgs},
    syn::{
        add_use_item,
        error::{
            err, AsBuildResult, BuildResult, MsgResult, SpannedError, SpannedResult,
            SplitBuildResult, ToError,
        },
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
    pub span: ErrorSpan,
}

#[derive(Debug)]
pub struct AstMod {
    pub idx: usize,
    pub ty: AstModType,
    pub file: PathBuf,
    pub path: Vec<String>,
    pub name: String,
    pub span: ErrorSpan,
    pub errs: Vec<SpannedError>,
    pub warns: Vec<SpannedError>,
    pub mods: Vec<usize>,
    pub uses: Vec<AstUse>,
    pub symbols: Vec<Symbol>,
    pub items: AstItems,
}

// TODO: ignore private mods to avoid name collisions
// Pass 1: parsing
impl AstMod {
    pub fn new(
        idx: usize,
        file: PathBuf,
        path: Vec<String>,
        ty: AstModType,
        span: ErrorSpan,
    ) -> MsgResult<Self> {
        Ok(Self {
            idx,
            ty,
            file,
            path,
            name: path
                .last()
                .catch_err(format!("Mod path is empty: {}", path.join("::")))?
                .to_string(),
            span,
            errs: Vec::new(),
            warns: Vec::new(),
            mods: Vec::new(),
            uses: Vec::new(),
            symbols: Vec::new(),
            items: AstItems::new(),
        })
    }

    pub fn parse(file: PathBuf, path: Vec<String>, ty: AstModType) -> MsgResult<Tree<Self>> {
        let file_contents = fs::read_to_string(file.to_owned())
            .catch_err(format!("Failed to read file: {}", file.display()))?;
        let ast = syn::parse_file(&file_contents).catch_err(format!(
            "Failed to parse file contents of: {}",
            file.display()
        ))?;

        let mut s = Self::new(0, file, path, ty, (&ast).into())?;

        let mods = s.visit_items(ast.items)?;
        // Resolve local use paths
        // E.g. mod Foo; use Foo::Bar;
        for use_path in s.uses.iter_mut() {
            if let Some(m) = mods
                .iter()
                .find(|m| m.root.path.last().is_some_and(|p| p == &use_path.first))
            {
                use_path.path = [m.root.path.to_vec(), use_path.path[1..].to_vec()].concat();
            }
        }

        Ok(Tree {
            root: s,
            children: mods,
        })
    }

    pub fn add_mod(&mut self, mods: &mut Vec<AstMod>, data: NewMod) -> MsgResult<()> {
        let mut new_mod = AstMod::new(
            0,
            self.file.clone(),
            self.path.to_vec().push_into(data.name),
            AstModType::Internal,
            data.span,
        )?;
        new_mod.symbols = data.symbols;
        new_mod.uses = data.uses;
        for m in data.mods {
            new_mod.add_mod(mods, m);
        }
        self.mods.push(mods.len());
        mods.push(new_mod);
        Ok(())
    }

    pub fn add_symbol(&mut self, symbol: Symbol) {
        self.symbols.push(symbol)
    }

    pub fn get_file(&self) -> String {
        self.file.display().to_string()
    }

    pub fn error(&mut self, msg: &str, span: impl Into<ErrorSpan>) {
        self.errs.push(err(msg, span));
    }

    pub fn warn(&mut self, msg: &str, span: impl Into<ErrorSpan>) {
        self.warns.push(err(msg, span))
    }

    pub fn take_errors(&mut self) -> Vec<SpannedError> {
        std::mem::replace(&mut self.errs, Vec::new())
    }

    // Iteration
    pub fn iter_structs_enums(&self) -> impl Iterator<Item = (&AstItemData, &Vec<AstAttribute>)> {
        self.items
            .structs
            .iter()
            .map(|s| (&s.data, &s.attrs))
            .chain(self.items.enums.iter().map(|e| (&e.data, &e.attrs)))
    }
}

// File/items
impl AstMod {
    fn visit_items(&mut self, items: Vec<syn::Item>) -> MsgResult<Vec<Tree<Self>>> {
        items
            .try_for_each(|i| {
                match i {
                    syn::Item::Use(i) => self.visit_item_use(i),
                    syn::Item::Fn(i) => self.visit_item_fn(i),
                    syn::Item::Enum(i) => self.visit_item_enum(i),
                    syn::Item::Struct(i) => self.visit_item_struct(i),
                    syn::Item::Macro(i) => self.visit_item_macro(i),
                    syn::Item::Mod(i) => {
                        return self.visit_item_mod(i).record_spanned_errs(&mut self.errs);
                    }
                    _ => Ok(()),
                }
                .record_errs(&mut self.errs);
                Ok(None)
            })
            .call_into(|(mods, errs)| errs.err_or(mods.filter_map_vec_into(|t| t)))
    }

    // Mod
    fn visit_item_mod(&mut self, i: syn::ItemMod) -> BuildResult<Option<Tree<Self>>> {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new()).upcast()? {
            match i.content {
                // Parse inner mod
                Some((_, items)) => {
                    let mut new_mod = Self::new(
                        0,
                        self.file.to_owned(),
                        [self.path.to_vec(), vec![i.ident.to_string()]].concat(),
                        AstModType::Internal,
                        self.span,
                    )
                    .upcast()?;
                    new_mod.visit_items(items).map(|mods| Tree {
                        root: new_mod,
                        children: mods,
                    })
                }
                // Parse file mod
                None => Self::parse_mod(
                    self.file
                        .to_owned()
                        .pop_into()
                        .push_into(i.ident.to_string()),
                    &[self.path.to_vec(), vec![i.ident.to_string()]].concat(),
                ),
            }
            .map(|m| Some(m))
            .upcast()
        } else {
            Ok(None)
        }
    }
}
