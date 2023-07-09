use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    path::PathBuf,
};

use quote::ToTokens;
use shared::{
    parse_args::SystemMacroArgs,
    util::{Call, Catch, JoinMap, JoinMapInto, PushInto, ThenOk},
};
use syn::parse::Lookahead1;

use crate::{
    parse::{AstCrate, AstMod, DiscardSymbol, HardcodedSymbol, MatchSymbol, ModInfo},
    resolve::util::{ItemIndex, MsgsResult},
    resolve::{
        path::{resolve_path, ItemPath},
        paths::EnginePaths,
    },
    util::{parse_syn_path, TAB},
};

use super::{
    component_set::{ComponentSet, LabelItem, LabelOp},
    items_crate::{ItemComponent, ItemGlobal, Items},
    labels::MustBe,
    path::{resolve_syn_path, ResolveResultTrait},
    paths::GetPaths,
    util::{CombineMsgs, MsgTrait, Zip2Msgs},
    ComponentSetLabels, ItemEvent, ItemSystem,
};

pub struct EventFnArg {
    pub arg_idx: usize,
    pub idx: usize,
}

pub struct GlobalFnArg {
    pub arg_idx: usize,
    pub idx: usize,
    pub is_mut: bool,
}

pub struct ComponentSetFnArg {
    pub arg_idx: usize,
    pub idx: usize,
    pub is_vec: bool,
}

pub enum FnArgs {
    Init {
        globals: Vec<GlobalFnArg>,
    },
    System {
        event: EventFnArg,
        globals: Vec<GlobalFnArg>,
        component_sets: Vec<ComponentSetFnArg>,
    },
}

pub struct ComponentRefTracker {
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

    pub fn validate(&self, comp_name: String) -> MsgsResult<()> {
        let (mut_cnt, immut_cnt) = (self.mut_refs.len(), self.immut_refs.len());

        (mut_cnt > 1)
            .err(
                vec![format!("Multiple mutable references to '{comp_name}'")],
                (),
            )
            .and_msgs((mut_cnt > 0 && immut_cnt > 0).err(
                vec![format!("Mutable and immutable references to '{comp_name}'")],
                (),
            ))
            .map_err(|mut errs| {
                if !self.mut_refs.is_empty() {
                    errs.push(format!(
                        "{TAB}Mutable references in '{}'",
                        self.mut_refs.join("', '")
                    ));
                }

                if !self.immut_refs.is_empty() {
                    errs.push(format!(
                        "{TAB}Immutable references in '{}'",
                        self.immut_refs.join("', '")
                    ));
                }
                errs
            })
    }
}

impl ItemSystem {
    pub fn validate(&self, items: &Items) -> MsgsResult<FnArgs> {
        let mut global_idxs = HashSet::new();

        if self.attr_args.is_init {
            self.args
                .enumerate_map_vec(|(arg_idx, arg)| match &arg.ty {
                    FnArgType::Global(idx) => {
                        Self::validate_global(arg, *idx, &mut global_idxs, items).map(|_| {
                            GlobalFnArg {
                                arg_idx,
                                idx: *idx,
                                is_mut: arg.is_mut,
                            }
                        })
                    }
                    FnArgType::Event(_) | FnArgType::Entities { .. } => Err(vec![format!(
                        "Init systems may not contain {}: {arg}",
                        arg.ty
                    )]),
                })
                .combine_msgs()
                .map(|globals| FnArgs::Init { globals })
        } else {
            let mut component_refs = HashMap::new();

            let mut event = None;
            let mut globals = Vec::new();
            let mut component_sets = Vec::new();
            self.args
                .enumerate_map_vec(|(arg_idx, arg)| match &arg.ty {
                    FnArgType::Event(idx) => match event {
                        Some(_) => Err(vec![format!("Multiple events specified: {arg}")]),
                        None => Self::validate_event(arg, *idx, items)
                            .map(|_| event = Some(EventFnArg { arg_idx, idx: *idx })),
                    },
                    FnArgType::Global(idx) => {
                        Self::validate_global(arg, *idx, &mut global_idxs, items).map(|_| {
                            globals.push(GlobalFnArg {
                                arg_idx,
                                idx: *idx,
                                is_mut: arg.is_mut,
                            });
                        })
                    }
                    FnArgType::Entities { idx, is_vec } => {
                        Self::validate_component_set(arg, *idx, *is_vec, &mut component_refs, items)
                            .map(|_| {
                                component_sets.push(ComponentSetFnArg {
                                    arg_idx,
                                    idx: *idx,
                                    is_vec: *is_vec,
                                });
                            })
                    }
                })
                .combine_msgs()
                // Require event
                .then_msgs(event.ok_or(vec![format!("System must specify an event")]))
                // Check component reference mutability
                .and_msgs(
                    component_refs
                        .into_iter()
                        .map_vec_into(|(i, refs)| {
                            items
                                .components
                                .get(i)
                                .ok_or(vec![format!("Invalid Component index: {i}")])
                                .and_then(|c| refs.validate(c.path.path.join("::")))
                        })
                        .combine_msgs(),
                )
                .map(|event| FnArgs::System {
                    event,
                    globals,
                    component_sets,
                })
        }
        .map_err(|mut errs| {
            errs.insert(0, format!("\nIn system: '{}':", self.path.path.join("::")));
            errs
        })
    }

    fn validate_global<'a>(
        arg: &FnArg,
        i: usize,
        globals: &mut HashSet<usize>,
        items: &'a Items,
    ) -> MsgsResult<&'a ItemGlobal> {
        items
            .globals
            .get(i)
            .ok_or(vec![format!("Invalid Global index: {i}")])
            .and_then(|g| {
                if g.args.is_const {
                    Self::validate_mut(arg, false).map(|_| g)
                } else {
                    Ok(g)
                }
            })
            .and_msgs(Self::validate_ref(arg, 1))
            .and_msgs(
                globals
                    .insert(i)
                    .ok((), vec![format!("Duplicate global: {arg}")]),
            )
    }

    fn validate_event<'a>(arg: &FnArg, i: usize, items: &'a Items) -> MsgsResult<&'a ItemEvent> {
        items
            .events
            .get(i)
            .ok_or(vec![format!("Invalid Event index: {i}")])
            .and_msgs(Self::validate_ref(arg, 1))
            .and_msgs(Self::validate_mut(arg, false))
    }

    fn validate_component_set<'a>(
        arg: &FnArg,
        i: usize,
        is_vec: bool,
        component_refs: &mut HashMap<usize, ComponentRefTracker>,
        items: &'a Items,
    ) -> MsgsResult<&'a ComponentSet> {
        let entities = EnginePaths::Entities.get_ident();

        items
            .component_sets
            .get(i)
            .ok_or(vec![format!("Invalid component set index: {i}")])
            .and_then(|cs| {
                let path = cs.path.path.join("::");

                // Add component refs
                for item in cs.args.iter() {
                    if !component_refs.contains_key(&item.comp.idx) {
                        component_refs
                            .insert(item.comp.idx, ComponentRefTracker::new());
                    }

                    if let Some(refs) = component_refs.get_mut(&item.comp.idx) {
                        refs.add_ref(path.to_string(), item.is_mut);
                    }
                }

                match is_vec || cs.first_singleton().is_some() {
                    true => Ok(cs),
                    // Must have a required singleton in the labels
                    false => {
                        let mut errs = vec![format!("{TAB}Entity set must contain singletons or be wrapped with {entities}<>")];
                        if let Some(ComponentSetLabels::Expression(e)) = &cs.labels {
                            for (symbs, verb) in [(&e.false_symbols, "must"), (&e.unknown_symbols, "may")] {
                                let comps = symbs
                                    .filter_map_vec(|c_sym|
                                        c_sym.args.is_singleton
                                            .then_some(items.components.get(c_sym.idx)
                                            .map_or_else(|| "Unknown".to_string(), 
                                            |c| c.path.path.join("::")
                                    )));
                                if !comps.is_empty() {
                                    errs.push(format!("{TAB}{TAB}These singleton are included as labels but {verb} be false: {}", comps.join(", ")))
                                }
                            }
                        }
                        Err(errs)
                    },
                }.map_err(|mut errs| {
                    [vec![format!("In entity set argument: {path} {{")], errs.push_into("}".to_string())].concat()
                })
            })
            .and_msgs(Self::validate_ref(arg, 0))
    }

    // Validate conditions
    fn validate_ref(arg: &FnArg, should_be_cnt: usize) -> MsgsResult<()> {
        (arg.ref_cnt == should_be_cnt).ok(
            (),
            vec![format!(
                "Type should be taken by {}: \"{}\"",
                if should_be_cnt == 0 {
                    "borrow".to_string()
                } else if should_be_cnt == 1 {
                    "single reference".to_string()
                } else {
                    format!("{} references", should_be_cnt)
                },
                arg
            )],
        )
    }

    fn validate_mut(arg: &FnArg, should_be_mut: bool) -> MsgsResult<()> {
        (arg.is_mut == should_be_mut).ok(
            (),
            vec![format!(
                "Type should be taken {}: \"{}\"",
                if should_be_mut {
                    "mutably"
                } else {
                    "immutably"
                },
                arg
            )],
        )
    }
}

#[derive(Clone, Debug)]
pub enum FnArgType {
    Event(usize),
    Global(usize),
    Entities { idx: usize, is_vec: bool },
}

impl std::fmt::Display for FnArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            match self {
                FnArgType::Event(i) => format!("Event {i}"),
                FnArgType::Global(i) => format!("Global {i}"),
                FnArgType::Entities { idx, is_vec } => format!(
                    "{} {idx}",
                    if *is_vec { "Vec<Entities>" } else { "Entities" }
                ),
            }
            .as_str(),
        )
    }
}

#[derive(Debug)]
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
        sig.inputs
            .iter()
            .map_vec_into(|arg| match arg {
                syn::FnArg::Receiver(_) => Err(vec!["Cannot use self in function".to_string()]),
                syn::FnArg::Typed(syn::PatType { ty, .. }) => {
                    FnArg::parse_type(ty, (m, cr, crates))
                }
            })
            .combine_msgs()
            .map_err(|mut errs| {
                errs.insert(
                    0,
                    format!(
                        "In system: '{}':",
                        m.path.to_vec().push_into(sig.ident.to_string()).join("::")
                    ),
                );
                errs
            })
    }

    fn parse_type(ty: &syn::Type, (m, cr, crates): ModInfo) -> MsgsResult<Self> {
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
                        crate::parse::SymbolType::Global(g_sym) => Ok(FnArgType::Global(g_sym.idx)),
                        crate::parse::SymbolType::Event(i) => Ok(FnArgType::Event(i)),
                        crate::parse::SymbolType::ComponentSet(idx) => {
                            Ok(FnArgType::Entities { idx, is_vec: false })
                        }
                        crate::parse::SymbolType::Hardcoded(s) => match s {
                            HardcodedSymbol::Entities => {
                                match generics.as_ref().and_then(|v| v.first()) {
                                    Some(syn::GenericArgument::Type(syn::Type::Path(ty))) => {
                                        resolve_syn_path(&m.path, &ty.path, (m, cr, crates))
                                            .expect_symbol()
                                            .expect_component_set()
                                            .discard_symbol()
                                            .map(|idx| FnArgType::Entities { idx, is_vec: true })
                                    }
                                    _ => Err(vec![format!(
                                        "Invalid argument type: {}",
                                        sym.path.join("::")
                                    )]),
                                }
                            }
                            _ => Err(vec![format!(
                                "Invalid argument type: {}",
                                sym.path.join("::")
                            )]),
                        },
                        _ => Err(vec![format!(
                            "Invalid argument type: {}",
                            sym.path.join("::")
                        )]),
                    })
                    .map(|data| Self {
                        ty: data,
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
                    Err(vec![format!(
                        "Trait arguments may only have one trait type: {ty_str}"
                    )])
                } else {
                    resolve_syn_path(&m.path, &traits[0].path, (m, cr, crates))
                        .expect_symbol()
                        .expect_trait()
                        .discard_symbol()
                        .map(|g_sym| Self {
                            ty: FnArgType::Global(g_sym.idx),
                            is_mut: false,
                            ref_cnt: 0,
                        })
                }
            }
            _ => Err(vec![format!("Invalid argument type: {ty_str}")]),
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
