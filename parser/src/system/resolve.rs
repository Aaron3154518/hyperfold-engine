use diagnostic::{CombineResults, ResultsTrait, ToErr};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

use shared::{
    constants::TAB,
    parsing::SystemMacroArgs,
    syn::error::{CriticalResult, ToError},
    traits::{CollectVec, CollectVecInto, PushInto, ThenOk},
};

use crate::{
    component_set::{ComponentSet, ComponentSetLabels},
    parse::ItemSpan,
    resolve::{ItemEvent, ItemGlobal, Items},
    utils::paths::ENGINE_PATHS,
};

use super::{
    parse::{FnArg, FnArgType},
    ItemSystem,
};

#[derive(Clone, Copy)]
pub struct EventFnArg {
    pub arg_idx: usize,
    pub idx: usize,
}

#[derive(Clone, Copy)]
pub struct GlobalFnArg {
    pub arg_idx: usize,
    pub idx: usize,
    pub is_mut: bool,
}

#[derive(Clone, Copy)]
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

struct ComponentRef {
    set_span: Span,
    item_span: Span,
}

pub struct ComponentRefTracker {
    comp_name: String,
    mut_refs: Vec<ComponentRef>,
    immut_refs: Vec<ComponentRef>,
}

impl ComponentRefTracker {
    pub fn new(comp_name: String) -> Self {
        Self {
            comp_name,
            mut_refs: Vec::new(),
            immut_refs: Vec::new(),
        }
    }

    pub fn with_ref(mut self, set_span: Span, item_span: Span, is_mut: bool) -> Self {
        self.add_ref(set_span, item_span, is_mut);
        self
    }

    pub fn add_ref(&mut self, set_span: Span, item_span: Span, is_mut: bool) {
        if is_mut {
            &mut self.mut_refs
        } else {
            &mut self.immut_refs
        }
        .push(ComponentRef {
            set_span,
            item_span,
        });
    }

    fn iter_refs(&self) -> impl Iterator<Item = (&ComponentRef, bool)> {
        self.mut_refs
            .iter()
            .map(|r| (r, true))
            .chain(self.immut_refs.iter().map(|r| (r, false)))
    }

    pub fn validate(&self, sys: &ItemSystem) -> CriticalResult<()> {
        let (mut_cnt, immut_cnt) = (self.mut_refs.len(), self.immut_refs.len());

        // TODO: one error per argument
        // TODO: remove duplicate items (e.g. same argument twice)
        (mut_cnt > 0 && immut_cnt > 0)
            .then_err(
                || {
                    let notes = || {
                        self.iter_refs().map(|(r, is_mut)| {
                            (
                                (&r.item_span).into(),
                                format!(
                                    "{} reference here",
                                    if is_mut { "Mutable" } else { "Immutable" }
                                ),
                            )
                        })
                    };
                    self.iter_refs().map_vec_into(|(r, is_mut)| {
                        r.set_span
                            .error(format!(
                                "Cannot have both mutable and immutable references to {}",
                                self.comp_name
                            ))
                            .with_notes(notes())
                    })
                },
                || (),
            )
            .and_then(|_| {
                let notes = || {
                    self.mut_refs
                        .iter()
                        .map(|r| ((&r.item_span).into(), "Mutable reference here".to_string()))
                };
                (mut_cnt > 1).err(
                    self.mut_refs.map_vec(|r| {
                        r.set_span
                            .error(format!(
                                "Cannot have multiple mutable references to {}",
                                self.comp_name
                            ))
                            .with_notes(notes())
                    }),
                    (),
                )
            })
        // TODO: add hint
        // .map_err(|mut errs| {
        //     if !self.mut_refs.is_empty() {
        //         errs.push(format!(
        //             "{TAB}Mutable references in '{}'",
        //             self.mut_refs.join("', '")
        //         ));
        //     }

        //     if !self.immut_refs.is_empty() {
        //         errs.push(format!(
        //             "{TAB}Immutable references in '{}'",
        //             self.immut_refs.join("', '")
        //         ));
        //     }
        //     errs
        // })
    }
}

impl ItemSystem {
    pub fn validate(&self, items: &Items) -> CriticalResult<FnArgs> {
        let mut global_idxs = HashSet::new();

        match self.attr_args {
            SystemMacroArgs::Init() => self
                .args
                .enumer_map_vec(|(arg_idx, arg)| match &arg.ty {
                    FnArgType::Global(idx) => self
                        .validate_global(arg, *idx, &mut global_idxs, items)
                        .map(|_| GlobalFnArg {
                            arg_idx,
                            idx: *idx,
                            is_mut: arg.is_mut,
                        }),
                    FnArgType::Event(_) | FnArgType::Entities { .. } => arg
                        .span
                        .error(format!("Init systems may not contain {}", arg.ty))
                        .as_err(),
                })
                .combine_results()
                .map(|globals| FnArgs::Init { globals }),
            SystemMacroArgs::System { .. } => {
                let mut component_refs = HashMap::new();

                let mut event = None;
                let mut globals = Vec::new();
                let mut component_sets = Vec::new();
                self.args
                    .enumer_map_vec(|(arg_idx, arg)| match &arg.ty {
                        FnArgType::Event(idx) => match event {
                            Some(_) => arg.span.error("Event already specified").as_err(),
                            None => self
                                .validate_event(arg, *idx, items)
                                .map(|_| event = Some(EventFnArg { arg_idx, idx: *idx })),
                        },
                        FnArgType::Global(idx) => self
                            .validate_global(arg, *idx, &mut global_idxs, items)
                            .map(|_| {
                                globals.push(GlobalFnArg {
                                    arg_idx,
                                    idx: *idx,
                                    is_mut: arg.is_mut,
                                });
                            }),
                        FnArgType::Entities { idx, is_vec } => self
                            .validate_component_set(arg, *idx, *is_vec, &mut component_refs, items)
                            .map(|_| {
                                component_sets.push(ComponentSetFnArg {
                                    arg_idx,
                                    idx: *idx,
                                    is_vec: *is_vec,
                                });
                            }),
                    })
                    .combine_results()
                    // Require event
                    .take_value(
                        event.ok_or(self.span.error("System must specify an event").as_vec()),
                    )
                    // Check component reference mutability
                    .take_errs(
                        component_refs
                            .into_iter()
                            .map_vec_into(|(i, refs)| refs.validate(self))
                            .combine_results(),
                    )
                    .map(|event| FnArgs::System {
                        event,
                        globals,
                        component_sets,
                    })
            }
        }
    }

    fn validate_global<'a>(
        &self,
        arg: &FnArg,
        i: usize,
        globals: &mut HashSet<usize>,
        items: &'a Items,
    ) -> CriticalResult<&'a ItemGlobal> {
        items
            .globals
            .get(i)
            .ok_or(
                arg.span
                    .error(format!("Invalid Global index: {i}"))
                    .as_vec(),
            )
            .and_then(|g| {
                if g.args.is_const {
                    self.validate_mut(arg, false).map(|_| g)
                } else {
                    Ok(g)
                }
            })
            .take_errs(self.validate_ref(arg, 1))
            .take_errs(
                globals
                    .insert(i)
                    .ok((), arg.span.error("Duplicate global").as_vec()),
            )
    }

    fn validate_event<'a>(
        &self,
        arg: &FnArg,
        i: usize,
        items: &'a Items,
    ) -> CriticalResult<&'a ItemEvent> {
        items
            .events
            .get(i)
            .ok_or(arg.span.error(format!("Invalid Event index: {i}")).as_vec())
            .take_errs(self.validate_ref(arg, 1))
            .take_errs(self.validate_mut(arg, false))
    }

    fn validate_component_set<'a>(
        &self,
        arg: &FnArg,
        i: usize,
        is_vec: bool,
        component_refs: &mut HashMap<usize, ComponentRefTracker>,
        items: &'a Items,
    ) -> CriticalResult<&'a ComponentSet> {
        items
            .component_sets
            .get(i)
            .ok_or(
                arg.span
                    .error(format!("Invalid component set index: {i}"))
                    .as_vec(),
            )
            .and_then(|cs| {
                let path = cs.path.path.join("::");

                // Add component refs
                for item in cs.args.iter() {
                    if !component_refs.contains_key(&item.comp.idx) {
                        component_refs
                            .insert(item.comp.idx, ComponentRefTracker::new(item.ty.to_string()));
                    }

                    if let Some(refs) = component_refs.get_mut(&item.comp.idx) {
                        refs.add_ref(arg.span, item.span, item.is_mut);
                    }
                }

                match is_vec || cs.has_singleton() {
                    true => Ok(cs),
                    // Must have a required singleton in the labels
                    false => {
                        let err = arg
                            .span
                            .error("Entity set must contain singletons or be wrapped with Vec<>");
                        // TODO: add help
                        // if let Some(ComponentSetLabels::Expression(e)) = &cs.labels {
                        //     for (symbs, verb) in [(&e.false_symbols, "must"), (&e.unknown_symbols, "may")] {
                        //         let comps = symbs
                        //             .filter_map_vec(|c_sym|
                        //                 c_sym.args.is_singleton
                        //                     .then_some(items.components.get(c_sym.idx)
                        //                     .map_or_else(|| "Unknown".to_string(),
                        //                     |c| c.path.path.join("::")
                        //             )));
                        //         if !comps.is_empty() {
                        //             errs.push(format!("{TAB}{TAB}These singleton are included as labels but {verb} be false: {}", comps.join(", ")))
                        //         }
                        //     }
                        // }
                        err.as_err()
                    }
                }
            })
            .take_errs(self.validate_ref(arg, 0))
    }

    // Validate conditions
    fn validate_ref(&self, arg: &FnArg, should_be_cnt: usize) -> CriticalResult<()> {
        (arg.ref_cnt == should_be_cnt).ok(
            (),
            arg.span
                .error(format!(
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
                .as_vec(),
        )
    }

    fn validate_mut(&self, arg: &FnArg, should_be_mut: bool) -> CriticalResult<()> {
        (arg.is_mut == should_be_mut).ok(
            (),
            arg.span
                .error(format!(
                    "Type should be taken {}: \"{}\"",
                    if should_be_mut {
                        "mutably"
                    } else {
                        "immutably"
                    },
                    arg
                ))
                .as_vec(),
        )
    }
}
