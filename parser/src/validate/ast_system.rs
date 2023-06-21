use std::collections::{HashMap, HashSet};

use shared::{
    parse_args::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs},
    util::JoinMap,
};

use crate::{
    codegen::system::LabelType,
    resolve::{
        function_arg::{FnArg, FnArgType},
        items_crate::{ItemSystem, ItemsCrate},
        path::ItemPath,
        paths::{EngineGlobals, EnginePaths, Paths},
    },
    validate::constants::{component_var, event_var},
};

use super::{
    constants::{event_variant, global_var, EID},
    util::ItemIndex,
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

    pub fn err(&self, comp_name: String, errs: &mut Vec<String>) {
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
    components: HashMap<ItemIndex, ComponentRefTracker>,
    globals: HashSet<ItemIndex>,
    has_event: bool,
    has_comp_set: bool,
}

impl SystemValidate {
    pub fn new() -> Self {
        Self {
            errs: Vec::new(),
            components: HashMap::new(),
            globals: HashSet::new(),
            has_event: false,
            has_comp_set: false,
        }
    }

    pub fn validate(&mut self, attr_args: &SystemMacroArgs, items: &Vec<ItemsCrate>) {
        for ((cr_i, c_i), refs) in self.components.iter() {
            refs.err(
                items[*cr_i].components[*c_i].path.path.join("::"),
                &mut self.errs,
            );
        }

        if attr_args.is_init {
            if !self.components.is_empty() {
                self.errs
                    .push("Init systems may not specify components".to_string());
            }
            if self.has_event {
                self.errs
                    .push("Init systems may not specify an event".to_string())
            }
        } else {
            if !self.has_event {
                self.errs
                    .push("Non-init systems must specify an event".to_string())
            }
        }
    }

    pub fn validate_global(&mut self, arg: &FnArg, idx: ItemIndex, args: &GlobalMacroArgs) {
        self.validate_ref(arg, 1);
        if args.is_const {
            self.validate_mut(arg, false);
        }
        if !self.globals.insert(idx) {
            self.errs.push(format!("Duplicate global: {arg}"));
        }
    }

    pub fn validate_event(&mut self, arg: &FnArg) {
        self.validate_ref(arg, 1);
        self.validate_mut(arg, false);
        if self.has_event {
            self.errs.push(format!("Multiple events specified: {arg}"));
        }
        self.has_event = true;
    }

    // pub fn validate_component_set(&mut self, arg: &FnArg, cs: &ComponentSet, is_vec: bool) {
    //     self.validate_ref(arg, 0);

    //     for i in cs.components.iter() {
    //         if !self.components.contains_key(&i.idx) {
    //             self.components.insert(i.idx, ComponentRefTracker::new());
    //         }

    //         if let Some(refs) = self.components.get_mut(&i.idx) {
    //             refs.add_ref(cs.path.path.join("::"), i.is_mut);
    //         }
    //     }

    //     if !is_vec && self.has_comp_set {
    //         self.errs.push(format!("Component set cannot be taken as an argument as another component set has already been specified: {arg}"));
    //         self.errs.push("\tConsider using 'Vec<>'".to_string());
    //     }
    //     self.has_comp_set = self.has_comp_set || is_vec;
    // }

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

impl FnArg {
    // pub fn validate(
    //     &self,
    //     crates: &Vec<ItemsCrate>,
    //     validate: &mut SystemValidate,
    // ) -> Option<FnArgResult> {
    // match &self.ty {
    //     // Global or Event or Component Set
    //     FnArgType::Path(p) => crates[p.cr_idx]
    //         // Global
    //         .find_global(p)
    //         .map(|(i, g)| {
    //             validate.validate_global(self, (p.cr_idx, i), &g.args);
    //             FnArgResult::Global {
    //                 idx: (p.cr_idx, i),
    //                 is_mut: self.is_mut,
    //             }
    //         })
    //         // Event
    //         .or_else(|| {
    //             crates[p.cr_idx].find_event(p).map(|(i, _)| {
    //                 validate.validate_event(self);
    //                 FnArgResult::Event((p.cr_idx, i))
    //             })
    //         })
    //         // Components Set
    //         .or_else(|| {
    //             // crates[p.cr_idx].find_component_set(p).map(|(i, cs)| {
    //             //     validate.validate_component_set(self, cs, false);
    //             //     FnArgResult::ComponentSet {
    //             //         idx: (p.cr_idx, i),
    //             //         is_vec: false,
    //             //     }
    //             // })
    //             None
    //         }),
    //     // Trait
    //     FnArgType::Trait(p) => crates[p.cr_idx].find_trait(p).map(|(_, tr)| {
    //         validate.validate_global(
    //             self,
    //             (tr.path.cr_idx, tr.g_idx),
    //             &GlobalMacroArgs::from(Vec::new()),
    //         );
    //         FnArgResult::Global {
    //             idx: (tr.path.cr_idx, tr.g_idx),
    //             is_mut: self.is_mut,
    //         }
    //     }),
    //     // Entities<Component Set>
    //     FnArgType::Entities(p) => None,
    //     // crates[p.cr_idx].find_component_set(p).map(|(i, cs)| {
    //     //     // validate.validate_component_set(self, cs, true);
    //     //     FnArgResult::ComponentSet {
    //     //         idx: (p.cr_idx, i),
    //     //         is_vec: true,
    //     //     }
    //     // }),
    // }
    // }
}
