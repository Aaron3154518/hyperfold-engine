use std::collections::HashSet;

use shared::{
    parse_args::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs},
    util::JoinMap,
};

use crate::{
    codegen::system::LabelType,
    parse::ast_fn_arg::{FnArg, FnArgType},
    resolve::{
        ast_items::{ItemsCrate, System},
        ast_paths::{EngineGlobals, EngineIdents, Paths},
        ast_resolve::Path,
    },
    validate::constants::{component_var, event_var},
};

use super::constants::{event_variant, global_var, EID};

pub struct SystemValidate {
    pub errs: Vec<String>,
    components: HashSet<(usize, usize)>,
    globals: HashSet<(usize, usize)>,
    has_event: bool,
    has_eid: bool,
    has_labels: bool,
    has_vec: bool,
}

impl SystemValidate {
    pub fn new() -> Self {
        Self {
            errs: Vec::new(),
            components: HashSet::new(),
            globals: HashSet::new(),
            has_event: false,
            has_eid: false,
            has_labels: false,
            has_vec: false,
        }
    }

    pub fn validate(&mut self, attr_args: &SystemMacroArgs) {
        if self.has_eid && self.components.is_empty() {
            self.errs
                .push("Cannot take entity ID without any entity components".to_string());
        }
        if self.has_vec && (!self.components.is_empty() || self.has_eid) {
            self.errs
                .push("Cannot wrap components in a vector and take them individually".to_string());
        }
        if attr_args.is_init
            && (self.has_eid
                || !self.components.is_empty()
                || self.has_labels
                || self.has_vec
                || self.has_event)
        {
            self.errs
                .push("Init systems may only take Globals".to_string());
        }
        if !attr_args.is_init && !self.has_event {
            self.errs
                .push("Non-init systems must specify and event".to_string())
        }
    }

    pub fn add_eid(&mut self) {
        self.has_eid = true;
    }

    pub fn add_component(&mut self, arg: &FnArg, idxs: (usize, usize)) {
        if !self.components.insert(idxs) {
            self.errs.push(format!("Duplicate component: {arg}"));
        }
    }

    pub fn add_global(&mut self, arg: &FnArg, idxs: (usize, usize)) {
        if !self.globals.insert(idxs) {
            self.errs.push(format!("Duplicate global: {arg}"));
        }
    }

    pub fn add_event(&mut self, arg: &FnArg) {
        if self.has_event {
            self.errs.push(format!("Multiple events specified: {arg}"));
        }
        self.has_event = true;
    }

    pub fn add_container(&mut self, arg: &FnArg) {
        if self.has_vec {
            self.errs
                .push(format!("Multiple containers specified: {arg}"))
        }
        self.has_vec = true;
    }
}

impl FnArg {
    // Convert to data
    pub fn to_eid(&self, paths: &Paths) -> Option<()> {
        matches!(&self.ty, FnArgType::Path(p) if p == paths.get_ident(EngineIdents::Entity))
            .then_some(())
    }

    pub fn to_component<'a>(
        &self,
        crates: &'a Vec<ItemsCrate>,
    ) -> Option<(usize, usize, &'a ComponentMacroArgs)> {
        match &self.ty {
            FnArgType::Path(p) => crates[p.cr_idx]
                .components
                .iter()
                .enumerate()
                .find_map(|(i, c)| (&c.path == p).then_some((p.cr_idx, i, &c.args))),
            _ => None,
        }
    }

    pub fn to_global<'a>(
        &self,
        crates: &'a Vec<ItemsCrate>,
    ) -> Option<(usize, usize, &'a GlobalMacroArgs)> {
        match &self.ty {
            FnArgType::Path(p) => crates[p.cr_idx]
                .globals
                .iter()
                .enumerate()
                .find_map(|(i, g)| (&g.path == p).then_some((p.cr_idx, i, &g.args))),
            _ => None,
        }
    }

    pub fn to_trait(&self, crates: &Vec<ItemsCrate>) -> Option<(usize, usize)> {
        match &self.ty {
            FnArgType::Trait(p) => crates[p.cr_idx]
                .traits
                .iter()
                .find_map(|tr| (tr.path.path == p.path).then_some((tr.path.cr_idx, tr.g_idx))),
            _ => None,
        }
    }

    pub fn to_event(&self, crates: &Vec<ItemsCrate>) -> Option<(usize, usize)> {
        match &self.ty {
            FnArgType::Path(p) => crates[p.cr_idx]
                .events
                .iter()
                .enumerate()
                .find_map(|(i, e)| (&e.path == p).then_some((p.cr_idx, i))),
            _ => None,
        }
    }

    pub fn to_label(&self, paths: &Paths) -> Option<(LabelType, Vec<&FnArg>)> {
        match &self.ty {
            FnArgType::SContainer(p, a) => {
                (p == paths.get_ident(EngineIdents::Label)).then(|| (LabelType::And, vec![&**a]))
            }
            FnArgType::Container(p, v) => (p == paths.get_ident(EngineIdents::AndLabels))
                .then_some(LabelType::And)
                .or_else(|| (p == paths.get_ident(EngineIdents::OrLabels)).then_some(LabelType::Or))
                .or_else(|| {
                    (p == paths.get_ident(EngineIdents::NandLabels)).then_some(LabelType::Nand)
                })
                .or_else(|| {
                    (p == paths.get_ident(EngineIdents::NorLabels)).then_some(LabelType::Nor)
                })
                .map(|l_ty| (l_ty, v.iter().collect())),
            _ => None,
        }
    }

    pub fn to_container(&self, paths: &Paths) -> Option<Vec<&FnArg>> {
        match &self.ty {
            FnArgType::Container(p, v) => {
                (p == paths.get_ident(EngineIdents::Container)).then_some(v.iter().collect())
            }
            _ => None,
        }
    }

    // Validate conditions
    pub fn validate_ref(&self, should_be_cnt: usize, errs: &mut Vec<String>) {
        if self.ref_cnt != should_be_cnt {
            errs.push(format!(
                "Type should be taken by {}: \"{}\"",
                if should_be_cnt == 0 {
                    "borrow".to_string()
                } else if should_be_cnt == 1 {
                    "single reference".to_string()
                } else {
                    format!("{} references", should_be_cnt)
                },
                self
            ))
        }
    }

    pub fn validate_mut(&self, should_be_mut: bool, errs: &mut Vec<String>) {
        if self.mutable != should_be_mut {
            errs.push(format!(
                "Type should be taken {}: \"{}\"",
                if should_be_mut {
                    "mutably"
                } else {
                    "immutably"
                },
                self
            ))
        }
    }
}
