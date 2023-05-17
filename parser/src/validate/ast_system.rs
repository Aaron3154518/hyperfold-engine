use std::collections::HashSet;

use shared::{
    parse_args::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs},
    util::JoinMap,
};

use crate::{
    parse::ast_fn_arg::{FnArg, FnArgType},
    resolve::{
        ast_items::{ItemsCrate, System},
        ast_paths::{EngineGlobals, EngineIdents, Paths},
        ast_resolve::Path,
    },
    validate::constants::{component_var, event_var},
};

use super::{
    ast_item_list::ItemList,
    constants::{event_variant, global_var, EID},
};

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
    fn to_eid(&self, paths: &Paths) -> Option<()> {
        matches!(&self.ty, FnArgType::Path(p) if p == paths.get_ident(EngineIdents::Entity))
            .then_some(())
    }

    fn to_component<'a>(
        &self,
        items: &'a ItemList,
    ) -> Option<(usize, usize, &'a ComponentMacroArgs)> {
        match &self.ty {
            FnArgType::Path(p) => items.components[p.cr_idx]
                .iter()
                .enumerate()
                .find_map(|(i, c)| (&c.path == p).then_some((p.cr_idx, i, &c.args))),
            _ => None,
        }
    }

    fn to_global<'a>(&self, items: &'a ItemList) -> Option<(usize, usize, &'a GlobalMacroArgs)> {
        match &self.ty {
            FnArgType::Path(p) => items.globals[p.cr_idx]
                .iter()
                .enumerate()
                .find_map(|(i, g)| (&g.path == p).then_some((p.cr_idx, i, &g.args))),
            _ => None,
        }
    }

    fn to_trait(&self, items: &ItemList) -> Option<(usize, usize)> {
        match &self.ty {
            FnArgType::Trait(p) => items
                .traits
                .iter()
                .find_map(|tr| (tr.path.path == p.path).then_some((tr.cr_idx, tr.g_idx))),
            _ => None,
        }
    }

    fn to_event(&self, items: &ItemList) -> Option<(usize, usize)> {
        match &self.ty {
            FnArgType::Path(p) => items.events[p.cr_idx]
                .iter()
                .enumerate()
                .find_map(|(i, e)| (&e.path == p).then_some((p.cr_idx, i))),
            _ => None,
        }
    }

    fn to_label(&self, paths: &Paths) -> Option<(&str, Vec<&FnArg>)> {
        match &self.ty {
            FnArgType::SContainer(p, a) => {
                (p == paths.get_ident(EngineIdents::Label)).then(|| ("&", vec![&**a]))
            }
            FnArgType::Container(p, v) => (p == paths.get_ident(EngineIdents::AndLabels))
                .then_some("&")
                .or_else(|| (p == paths.get_ident(EngineIdents::OrLabels)).then_some("|"))
                .or_else(|| (p == paths.get_ident(EngineIdents::NandLabels)).then_some("|&"))
                .or_else(|| (p == paths.get_ident(EngineIdents::NorLabels)).then_some("!|"))
                .map(|l_ty| (l_ty, v.iter().collect())),
            _ => None,
        }
    }

    fn to_container(&self, paths: &Paths) -> Option<Vec<&FnArg>> {
        match &self.ty {
            FnArgType::Container(p, v) => {
                (p == paths.get_ident(EngineIdents::Container)).then_some(v.iter().collect())
            }
            _ => None,
        }
    }

    // Validate conditions
    fn validate_ref(&self, should_be_cnt: usize, errs: &mut Vec<String>) {
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

    fn validate_mut(&self, should_be_mut: bool, errs: &mut Vec<String>) {
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

    pub fn validate_to_data(
        &self,
        paths: &Paths,
        items: &ItemList,
        validate: &mut SystemValidate,
    ) -> String {
        match self
            // Entity ID
            .to_eid(paths)
            .map(|_| {
                self.validate_ref(1, &mut validate.errs);
                self.validate_mut(false, &mut validate.errs);
                validate.add_eid();
                EID.to_string()
            })
            // Component
            .or_else(|| {
                self.to_component(items).map(|(cr_i, c_i, _)| {
                    self.validate_ref(1, &mut validate.errs);
                    validate.add_component(self, (cr_i, c_i));
                    component_var(cr_i, c_i)
                })
            })
            // Global
            .or_else(|| {
                self.to_global(items).map(|(cr_i, g_i, g_args)| {
                    self.validate_ref(1, &mut validate.errs);
                    if g_args.is_const {
                        self.validate_mut(false, &mut validate.errs);
                    }
                    validate.add_global(self, (cr_i, g_i));
                    global_var(cr_i, g_i)
                })
            })
            // Event
            .or_else(|| {
                self.to_event(items).map(|(cr_i, e_i)| {
                    self.validate_ref(1, &mut validate.errs);
                    self.validate_mut(false, &mut validate.errs);
                    validate.add_event(self);
                    event_variant(cr_i, e_i)
                })
            })
            // Trait
            .or_else(|| {
                // This assumes the traits are all at the beginning of the globals list
                self.to_trait(items).map(|(cr_i, g_i)| {
                    self.validate_ref(1, &mut validate.errs);
                    validate.add_global(self, (cr_i, g_i));
                    global_var(cr_i, g_i)
                })
            })
            // Label
            .or_else(|| {
                self.to_label(paths).map(|(l_ty, args)| {
                    self.validate_ref(0, &mut validate.errs);
                    format!(
                        "l{}{}",
                        l_ty,
                        args.join_map(
                            |a| a.to_component(items).map_or_else(
                                || {
                                    &mut validate.errs.push(format!(
                                        "Label expects Component type, found: {}",
                                        a
                                    ));
                                    String::new()
                                },
                                |(cr_i, c_i, _)| component_var(cr_i, c_i)
                            ),
                            "-"
                        )
                    )
                })
            })
            // Container
            .or_else(|| {
                self.to_container(paths).map(|args| {
                    self.validate_ref(0, &mut validate.errs);
                    validate.add_container(self);
                    format!(
                        "v{}",
                        args.join_map(
                            |a| a
                                .to_eid(paths)
                                .map(|_| {
                                    a.validate_ref(1, &mut validate.errs);
                                    a.validate_mut(false, &mut validate.errs);
                                    EID.to_string()
                                })
                                .or_else(|| a.to_component(items).map(|(cr_i, c_i, _)| {
                                    a.validate_ref(1, &mut validate.errs);
                                    format!(
                                        "{}{}",
                                        if a.mutable { "m" } else { "" },
                                        component_var(cr_i, c_i)
                                    )
                                }))
                                .map_or_else(
                                    || {
                                        &mut validate.errs.push(format!(
                                        "Container expects Component or Entity ID type, found: {}",
                                        a
                                    ));
                                        String::new()
                                    },
                                    |s| s
                                ),
                            "-"
                        )
                    )
                })
            }) {
            Some(data) => data,
            None => {
                validate
                    .errs
                    .push(format!("Argument: \"{}\" is not a known type", self));
                String::new()
            }
        }
    }
}

impl System {
    pub fn validate_to_data(
        &self,
        paths: &Paths,
        crates: &Vec<ItemsCrate>,
        items: &ItemList,
    ) -> String {
        let mut validate = SystemValidate::new();
        let data = format!(
            "{}({}){}",
            self.path.path[1..].join("::"),
            self.args
                .join_map(|a| a.validate_to_data(paths, items, &mut validate), ":"),
            if self.attr_args.is_init { "i" } else { "" }
        );
        validate.validate(&self.attr_args);
        if !validate.errs.is_empty() {
            panic!(
                "\n\nIn system: \"{}()\"\n{}\n\n",
                self.path.path.join("::"),
                validate.errs.join("\n")
            )
        }
        data
    }
}
