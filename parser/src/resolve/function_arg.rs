use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    path::PathBuf,
};

use quote::ToTokens;
use shared::{
    parse_args::SystemMacroArgs,
    util::{Call, Catch, JoinMap, PushInto, ThenOk},
};

use crate::{
    parse::{AstCrate, AstMod, DiscardSymbol, HardcodedSymbol, MatchSymbol, ModInfo},
    resolve::{
        path::{resolve_path, ItemPath},
        paths::{EnginePaths, Paths},
    },
    util::parse_syn_path,
    validate::util::{ItemIndex, MsgsResult},
};

use super::{
    component_set::ComponentSet,
    items_crate::{ItemComponent, ItemGlobal, Items},
    path::{resolve_syn_path, ResolveResultTrait},
};

struct ComponentRefTracker {
    mut_refs: Vec<String>,
    immut_refs: Vec<String>,
}

impl ComponentRefTracker {
    pub fn new() -> Self {
        Self {
            mut_refs: Vec::new(),
            immut_refs: Vec::new(),
        }
    }

    pub fn with_ref(mut self, set_name: String, is_mut: bool) -> Self {
        self.add_ref(set_name, is_mut);
        self
    }

    pub fn add_ref(&mut self, set_name: String, is_mut: bool) {
        if is_mut {
            self.mut_refs.push(set_name);
        } else {
            self.immut_refs.push(set_name);
        }
    }

    pub fn validate(&self, comp_name: String, errs: &mut Vec<String>) {
        let (mut_cnt, immut_cnt) = (self.mut_refs.len(), self.immut_refs.len());

        if mut_cnt > 1 {
            errs.push(format!("'{comp_name}' has multiple mutable references"));
        }

        if mut_cnt > 0 && immut_cnt > 0 {
            errs.push(format!(
                "'{comp_name}' has mutable and immutable references"
            ));
        }

        if mut_cnt > 1 || (mut_cnt > 0 && immut_cnt > 0) {
            self.note(errs)
        }
    }

    pub fn note(&self, errs: &mut Vec<String>) {
        if !self.mut_refs.is_empty() {
            errs.push(format!(
                "\tMutable references in '{}'",
                self.mut_refs.join("', '")
            ));
        }

        if !self.immut_refs.is_empty() {
            errs.push(format!(
                "\tImmutable references in '{}'",
                self.immut_refs.join("', '")
            ));
        }
    }
}

pub struct SystemValidate {
    pub errs: Vec<String>,
    attrs: SystemMacroArgs,
    components: HashMap<usize, ComponentRefTracker>,
    globals: HashSet<usize>,
    has_event: bool,
    has_comp_set: bool,
}

impl SystemValidate {
    pub fn new(attrs: SystemMacroArgs) -> Self {
        Self {
            errs: Vec::new(),
            attrs,
            components: HashMap::new(),
            globals: HashSet::new(),
            has_event: false,
            has_comp_set: false,
        }
    }

    pub fn validate(&mut self, path: String, items: &Items) -> Vec<String> {
        for (i, refs) in self.components.iter() {
            let c = match items.components.get(*i) {
                Some(c) => c,
                None => {
                    self.errs.push(format!("Invalid Component index: {i}"));
                    continue;
                }
            };
            refs.validate(c.path.path.join("::"), &mut self.errs);
        }

        if !self.attrs.is_init {
            if !self.has_event {
                self.errs
                    .push("Non-init systems must specify an event".to_string());
            }
        }

        if !self.errs.is_empty() {
            self.errs.insert(0, format!("In system: '{path}':"));
        }
        self.errs.to_vec()
    }

    pub fn validate_global(&mut self, arg: &FnArg, i: usize, items: &Items) {
        let g = match items.globals.get(i) {
            Some(g) => g,
            None => {
                self.errs.push(format!("Invalid Global index: {i}"));
                return;
            }
        };

        self.validate_ref(arg, 1);
        if g.args.is_const {
            self.validate_mut(arg, false);
        }
        if !self.globals.insert(i) {
            self.errs.push(format!("Duplicate global: {arg}"));
        }
    }

    pub fn validate_event(&mut self, arg: &FnArg) {
        if self.attrs.is_init {
            self.errs
                .push(format!("Init system may not specify an event: {arg}"));
            return;
        }

        self.validate_ref(arg, 1);
        self.validate_mut(arg, false);
        if self.has_event {
            self.errs.push(format!("Multiple events specified: {arg}"));
        }
        self.has_event = true;
    }

    pub fn validate_component_set(&mut self, arg: &FnArg, i: usize, items: &Items, is_vec: bool) {
        if self.attrs.is_init {
            self.errs
                .push(format!("Init system may not specify components: {arg}"));
            return;
        }

        let cs = match items.component_sets.get(i) {
            Some(cs) => cs,
            None => {
                self.errs.push(format!("Invalid Entities index: {i}"));
                return;
            }
        };

        self.validate_ref(arg, 0);

        for item in cs.args.iter() {
            if !self.components.contains_key(&item.c_idx) {
                self.components
                    .insert(item.c_idx, ComponentRefTracker::new());
            }

            if let Some(refs) = self.components.get_mut(&item.c_idx) {
                refs.add_ref(cs.path.path.join("::"), item.is_mut);
            }
        }

        if !is_vec && self.has_comp_set {
            self.errs.push(format!("Component set cannot be taken as an argument as another component set has already been specified: {arg}"));
            self.errs.push("\tConsider using 'Vec<>'".to_string());
        }
        self.has_comp_set = self.has_comp_set || is_vec;
    }

    // Validate conditions
    pub fn validate_ref(&mut self, arg: &FnArg, should_be_cnt: usize) {
        if arg.ref_cnt != should_be_cnt {
            self.errs.push(format!(
                "Type should be taken by {}: \"{}\"",
                if should_be_cnt == 0 {
                    "borrow".to_string()
                } else if should_be_cnt == 1 {
                    "single reference".to_string()
                } else {
                    format!("{} references", should_be_cnt)
                },
                arg
            ))
        }
    }

    pub fn validate_mut(&mut self, arg: &FnArg, should_be_mut: bool) {
        if arg.is_mut != should_be_mut {
            self.errs.push(format!(
                "Type should be taken {}: \"{}\"",
                if should_be_mut {
                    "mutably"
                } else {
                    "immutably"
                },
                arg
            ))
        }
    }
}

pub enum FnArgResult {
    Global { idx: ItemIndex, is_mut: bool },
    Event(ItemIndex),
    ComponentSet { idx: ItemIndex, is_vec: bool },
}

#[derive(Clone, Debug)]
pub enum FnArgType {
    Event(usize),
    Global(usize),
    Entities(usize),
    VecEntities(usize),
}

impl std::fmt::Display for FnArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            match self {
                FnArgType::Event(i) => format!("Event {i}"),
                FnArgType::Global(i) => format!("Global {i}"),
                FnArgType::Entities(i) => format!("Entities {i}"),
                FnArgType::VecEntities(i) => format!("Vec Entities {i}"),
            }
            .as_str(),
        )
    }
}

#[derive(Clone, Debug)]
pub struct FnArg {
    pub ty: FnArgType,
    pub is_mut: bool,
    pub ref_cnt: usize,
}

impl FnArg {
    pub fn parse(
        attrs: &SystemMacroArgs,
        items: &Items,
        sig: &syn::Signature,
        (m, cr, crates): ModInfo,
    ) -> MsgsResult<Vec<Self>> {
        let mut validator = SystemValidate::new(attrs.clone());
        // Parse
        let args = sig
            .inputs
            .iter()
            .filter_map(|arg| match arg {
                syn::FnArg::Receiver(_) => {
                    validator
                        .errs
                        .push("Cannot use self in function".to_string());
                    None
                }
                syn::FnArg::Typed(syn::PatType { ty, .. }) => {
                    match FnArg::parse_type(ty, (m, cr, crates)) {
                        Ok(ty) => Some(ty),
                        Err(e) => {
                            validator.errs.push(e);
                            None
                        }
                    }
                }
            })
            .collect::<Vec<_>>();
        // Validate
        for arg in args.iter() {
            match &arg.ty {
                FnArgType::Event(_) => validator.validate_event(arg),
                FnArgType::Global(i) => validator.validate_global(arg, *i, items),
                FnArgType::Entities(i) => validator.validate_component_set(arg, *i, items, false),
                FnArgType::VecEntities(i) => validator.validate_component_set(arg, *i, items, true),
            }
        }
        // Throw errors
        let errs = validator.validate(
            m.path.to_vec().push_into(sig.ident.to_string()).join("::"),
            items,
        );
        errs.is_empty().result(args, errs)
    }

    fn parse_type(ty: &syn::Type, (m, cr, crates): ModInfo) -> Result<Self, String> {
        let ty_str = ty.to_token_stream().to_string();
        match ty {
            syn::Type::Path(p) => {
                let generics = p.path.segments.last().and_then(|s| match &s.arguments {
                    syn::PathArguments::AngleBracketed(ab) => {
                        Some(ab.args.iter().collect::<Vec<_>>())
                    }
                    _ => None,
                });

                resolve_syn_path(&m.path, &p.path, (m, cr, crates))
                    .expect_symbol()
                    .and_then(|sym| match sym.kind {
                        crate::parse::SymbolType::Global(i) => Ok(FnArgType::Global(i)),
                        crate::parse::SymbolType::Event(i) => Ok(FnArgType::Event(i)),
                        crate::parse::SymbolType::ComponentSet(i) => Ok(FnArgType::Entities(i)),
                        crate::parse::SymbolType::Hardcoded(s) => match s {
                            HardcodedSymbol::Entities => {
                                if let Some(syn::GenericArgument::Type(syn::Type::Path(ty))) =
                                    generics.as_ref().and_then(|v| v.first())
                                {
                                    resolve_syn_path(&m.path, &ty.path, (m, cr, crates))
                                        .expect_symbol()
                                        .expect_component_set()
                                        .discard_symbol()
                                        .map(|i| FnArgType::VecEntities(i))
                                } else {
                                    Err(format!("Invalid argument type: {}", sym.path.join("::")))
                                }
                            }
                            _ => Err(format!("Invalid argument type: {}", sym.path.join("::"))),
                        },
                        _ => Err(format!("Invalid argument type: {}", sym.path.join("::"))),
                    })
                    .map(|ty| Self {
                        ty,
                        is_mut: false,
                        ref_cnt: 0,
                    })
            }
            syn::Type::Reference(r) => {
                Self::parse_type(&r.elem, (m, cr, crates)).map(|mut fn_arg| {
                    fn_arg.ref_cnt += 1;
                    fn_arg.is_mut = fn_arg.is_mut || r.mutability.is_some();
                    fn_arg
                })
            }
            syn::Type::TraitObject(t) => {
                let traits = t
                    .bounds
                    .iter()
                    .filter_map(|tpb| match tpb {
                        syn::TypeParamBound::Trait(tr) => Some(tr),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if traits.len() != 1 {
                    Err(format!(
                        "Trait arguments may only have one trait type: {ty_str}"
                    ))
                } else {
                    resolve_syn_path(&m.path, &traits[0].path, (m, cr, crates))
                        .expect_symbol()
                        .expect_trait()
                        .discard_symbol()
                        .map(|i| Self {
                            ty: FnArgType::Global(i),
                            is_mut: false,
                            ref_cnt: 0,
                        })
                }
            }
            _ => Err(format!("Invalid argument type: {ty_str}")),
        }
    }
}

impl std::fmt::Display for FnArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}{}{}",
            "&".repeat(self.ref_cnt),
            if self.is_mut { "mut " } else { "" },
            self.ty
        ))
    }
}
