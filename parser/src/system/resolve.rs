use std::collections::{HashMap, HashSet};

use proc_macro2::Span;
use shared::{
    constants::TAB,
    msg_result::{CombineMsgs, MsgTrait},
    traits::{CollectVec, CollectVecInto, PushInto, ThenOk},
};

use crate::{
    component_set::{ComponentSet, ComponentSetLabels},
    resolve::{ItemEvent, ItemGlobal, Items},
    utils::{paths::ENGINE_PATHS, Msg, MsgResult},
};

use super::{
    parse::{FnArg, FnArgType},
    ItemSystem,
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

    pub fn validate(&self, sys: &ItemSystem) -> MsgResult<()> {
        let (mut_cnt, immut_cnt) = (self.mut_refs.len(), self.immut_refs.len());

        (mut_cnt > 1)
            .err(vec![sys.msg("Multiple mutable references")], ())
            .and_msgs(
                (mut_cnt > 0 && immut_cnt > 0)
                    .err(vec![sys.msg("Mutable and immutable references")], ()),
            )
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
    pub fn validate(&self, items: &Items) -> MsgResult<FnArgs> {
        let mut global_idxs = HashSet::new();

        if self.attr_args.is_init {
            self.args
                .enumerate_map_vec(|(arg_idx, arg)| match &arg.ty {
                    FnArgType::Global(idx) => self
                        .validate_global(arg, *idx, &mut global_idxs, items)
                        .map(|_| GlobalFnArg {
                            arg_idx,
                            idx: *idx,
                            is_mut: arg.is_mut,
                        }),
                    FnArgType::Event(_) | FnArgType::Entities { .. } => Err(vec![
                        self.msg(&format!("Init systems may not contain {}", arg.ty))
                    ]),
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
                        Some(_) => Err(vec![self.new_msg("Event already specified", &arg.span)]),
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
                .combine_msgs()
                // Require event
                .then_msgs(event.ok_or(vec![self.msg("System must specify an event")]))
                // Check component reference mutability
                .and_msgs(
                    component_refs
                        .into_iter()
                        .map_vec_into(|(i, refs)| refs.validate(self))
                        .combine_msgs(),
                )
                .map(|event| FnArgs::System {
                    event,
                    globals,
                    component_sets,
                })
        }
    }

    fn validate_global<'a>(
        &self,
        arg: &FnArg,
        i: usize,
        globals: &mut HashSet<usize>,
        items: &'a Items,
    ) -> MsgResult<&'a ItemGlobal> {
        items
            .globals
            .get(i)
            .ok_or(vec![Msg::String(format!("Invalid Global index: {i}"))])
            .and_then(|g| {
                if g.args.is_const {
                    self.validate_mut(arg, false).map(|_| g)
                } else {
                    Ok(g)
                }
            })
            .and_msgs(self.validate_ref(arg, 1))
            .and_msgs(
                globals
                    .insert(i)
                    .ok((), vec![self.new_msg("Duplicate global", &arg.span)]),
            )
    }

    fn validate_event<'a>(
        &self,
        arg: &FnArg,
        i: usize,
        items: &'a Items,
    ) -> MsgResult<&'a ItemEvent> {
        items
            .events
            .get(i)
            .ok_or(vec![Msg::String(format!("Invalid Event index: {i}"))])
            .and_msgs(self.validate_ref(arg, 1))
            .and_msgs(self.validate_mut(arg, false))
    }

    fn validate_component_set<'a>(
        &self,
        arg: &FnArg,
        i: usize,
        is_vec: bool,
        component_refs: &mut HashMap<usize, ComponentRefTracker>,
        items: &'a Items,
    ) -> MsgResult<&'a ComponentSet> {
        items
            .component_sets
            .get(i)
            .ok_or(vec![Msg::String(format!(
                "Invalid component set index: {i}"
            ))])
            .and_then(|cs| {
                let path = cs.path.path.join("::");

                // Add component refs
                for item in cs.args.iter() {
                    if !component_refs.contains_key(&item.comp.idx) {
                        component_refs.insert(item.comp.idx, ComponentRefTracker::new());
                    }

                    if let Some(refs) = component_refs.get_mut(&item.comp.idx) {
                        refs.add_ref(path.to_string(), item.is_mut);
                    }
                }

                match is_vec || cs.has_singleton() {
                    true => Ok(cs),
                    // Must have a required singleton in the labels
                    false => {
                        let err = self.new_msg(
                            &format!("Entity set must contain singletons or be wrapped with Vec<>"),
                            &arg.span,
                        );
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
                        Err(vec![err])
                    }
                }
            })
            .and_msgs(self.validate_ref(arg, 0))
    }

    // Validate conditions
    fn validate_ref(&self, arg: &FnArg, should_be_cnt: usize) -> MsgResult<()> {
        (arg.ref_cnt == should_be_cnt).ok(
            (),
            vec![self.new_msg(
                &format!(
                    "Type should be taken by {}: \"{}\"",
                    if should_be_cnt == 0 {
                        "borrow".to_string()
                    } else if should_be_cnt == 1 {
                        "single reference".to_string()
                    } else {
                        format!("{} references", should_be_cnt)
                    },
                    arg
                ),
                &arg.span,
            )],
        )
    }

    fn validate_mut(&self, arg: &FnArg, should_be_mut: bool) -> MsgResult<()> {
        (arg.is_mut == should_be_mut).ok(
            (),
            vec![self.new_msg(
                &format!(
                    "Type should be taken {}: \"{}\"",
                    if should_be_mut {
                        "mutably"
                    } else {
                        "immutably"
                    },
                    arg
                ),
                &arg.span,
            )],
        )
    }
}
