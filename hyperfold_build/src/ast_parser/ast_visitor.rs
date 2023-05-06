use super::{
    component::{Component, Global},
    event::EventMod,
    system::{System, SystemArg},
    util::{end, get_attributes, join_paths, Attribute},
};
use crate::ast_parser::util::concatenate;
use hyperfold_shared::macro_args::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs};

// AstVisitor
pub struct AstVisitor {
    pub components: Vec<Component>,
    pub globals: Vec<Global>,
    pub modules: Vec<String>,
    pub systems: Vec<System>,
    pub events: Vec<EventMod>,
    pub path: Vec<String>,
    pub uses: Vec<(Vec<String>, String)>,
    pub build_use: Vec<String>,
}

impl AstVisitor {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            globals: Vec::new(),
            modules: Vec::new(),
            systems: Vec::new(),
            events: Vec::new(),
            // This is empty for main.rs
            path: Vec::new(),
            // Add use paths from using "crate::"
            uses: vec![(vec!["crate".to_string()], String::new())],
            build_use: Vec::new(),
        }
    }

    pub fn to_file(&self) -> String {
        format!(
            "src/{}.rs",
            if self.path.is_empty() {
                // Add main
                "main".to_string()
            } else {
                self.path.join("/")
            }
        )
    }

    pub fn get_mod_path(&self) -> Vec<String> {
        concatenate(vec!["crate".to_string()], self.path.to_vec())
    }

    pub fn add_component(&mut self, c: Component) {
        self.components.push(c);
    }

    pub fn add_global(&mut self, g: Global) {
        self.globals.push(g);
    }

    pub fn uses_string(&self) -> String {
        self.uses
            .iter()
            .map(|u| {
                format!(
                    "{}{}",
                    u.0.join("::"),
                    if u.1.is_empty() {
                        String::new()
                    } else {
                        format!(" as {}", u.1)
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n      ")
    }

    pub fn components_string(&self) -> String {
        join_paths(&self.components)
    }

    pub fn globals_string(&self) -> String {
        join_paths(&self.globals)
    }

    pub fn events_string(&self) -> String {
        join_paths(&self.events)
    }

    pub fn systems_string(&self) -> String {
        self.systems
            .iter()
            .map(|s| {
                format!(
                    "{}({})",
                    s.path.last().expect("Function has no name"),
                    s.args
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl syn::visit_mut::VisitMut for AstVisitor {
    fn visit_item_mod_mut(&mut self, i: &mut syn::ItemMod) {
        self.modules.push(i.ident.to_string());
        syn::visit_mut::visit_item_mod_mut(self, i);
    }

    // Components
    fn visit_item_struct_mut(&mut self, i: &mut syn::ItemStruct) {
        get_attributes(&i.attrs)
            .into_iter()
            .find_map(|(a, args)| match a {
                Attribute::Component => {
                    self.add_component(Component {
                        path: concatenate(self.get_mod_path(), vec![i.ident.to_string()]),
                        args: ComponentMacroArgs::from(args),
                    });
                    Some(())
                }
                Attribute::Global => {
                    self.add_global(Global {
                        path: concatenate(self.get_mod_path(), vec![i.ident.to_string()]),
                        args: GlobalMacroArgs::from(args),
                    });
                    Some(())
                }
                _ => None,
            });
        syn::visit_mut::visit_item_struct_mut(self, i);
    }

    // Functions
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        get_attributes(&i.attrs).iter().find_map(|(a, args)| {
            match a {
                Attribute::System => {
                    // Parse function path and args
                    self.systems.push(System {
                        path: concatenate(self.get_mod_path(), vec![i.sig.ident.to_string()]),
                        args: i
                            .sig
                            .inputs
                            .iter()
                            .filter_map(|arg| match arg {
                                syn::FnArg::Typed(t) => {
                                    let mut fn_arg = SystemArg::new();
                                    fn_arg.parse_arg(&self.get_mod_path(), &t);
                                    Some(fn_arg)
                                }
                                syn::FnArg::Receiver(_) => None,
                            })
                            .collect(),
                        macro_args: SystemMacroArgs::from(args.to_vec()),
                    });
                    Some(())
                }
                _ => None,
            }
        });
        syn::visit_mut::visit_item_fn_mut(self, i);
    }

    // Enums
    fn visit_item_enum_mut(&mut self, i: &mut syn::ItemEnum) {
        if let Some(_) = get_attributes(&i.attrs).iter().find_map(|(a, _)| match a {
            Attribute::Event => Some(()),
            _ => None,
        }) {
            self.events.push(EventMod {
                path: concatenate(self.get_mod_path(), vec![i.ident.to_string()]),
                events: i.variants.iter().map(|v| v.ident.to_string()).collect(),
            });
        }
        syn::visit_mut::visit_item_enum_mut(self, i);
    }

    // Use Statements
    fn visit_item_use_mut(&mut self, i: &mut syn::ItemUse) {
        self.build_use = Vec::new();
        syn::visit_mut::visit_item_use_mut(self, i);
    }

    fn visit_use_path_mut(&mut self, i: &mut syn::UsePath) {
        if i.ident == "super" {
            if self.build_use.is_empty() {
                self.build_use.push("crate".to_string());
                self.build_use
                    .append(&mut self.path[..end(&self.path, 0)].to_vec());
            } else {
                self.build_use.pop();
            }
        } else {
            self.build_use.push(i.ident.to_string());
        }
        syn::visit_mut::visit_use_path_mut(self, i);
        self.build_use.pop();
    }

    fn visit_use_name_mut(&mut self, i: &mut syn::UseName) {
        // Push
        self.build_use.push(i.ident.to_string());
        self.uses.push((self.build_use.to_vec(), String::new()));
        self.build_use.pop();
        syn::visit_mut::visit_use_name_mut(self, i);
    }

    fn visit_use_rename_mut(&mut self, i: &mut syn::UseRename) {
        self.build_use.push(i.rename.to_string());
        self.uses
            .push((self.build_use.to_vec(), i.ident.to_string()));
        self.build_use.pop();
        syn::visit_mut::visit_use_rename_mut(self, i);
    }

    fn visit_use_glob_mut(&mut self, i: &mut syn::UseGlob) {
        self.build_use.push("*".to_string());
        self.uses.push((self.build_use.to_vec(), String::new()));
        self.build_use.pop();
        syn::visit_mut::visit_use_glob_mut(self, i);
    }
}
