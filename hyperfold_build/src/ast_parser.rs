use hyperfold_shared::macro_args::GlobalMacroArgs;
use hyperfold_shared::paths::{COMPONENTS_MANAGER, COMPONENTS_TRAIT, EVENTS_MANAGER, EVENTS_TRAIT};

use crate::ast_visitor::AstVisitor;
use crate::component::{Component, Global, Trait};
use crate::event::EventMod;
use crate::system::System;
use crate::util::{concatenate, to_data};
use std::{fs::File, io::Read, path::Path};

// Parsing file
pub struct AstParser {
    prefixes: Vec<String>,
    files: Vec<AstVisitor>,
}

impl AstParser {
    pub fn new() -> Self {
        Self {
            prefixes: Vec::new(),
            files: Vec::new(),
        }
    }

    pub fn parse(
        &mut self,
        prefix: &str,
        deps: Vec<&str>,
        entry_file: &str,
        features: Vec<String>,
    ) {
        let p = Path::new(&entry_file);
        if !p.exists() {
            panic!("Could not open entry file for parsing: {}", entry_file);
        }
        let n = self.files.len();
        self.parse_file(
            p.parent()
                .expect(format!("Could not parse directory of entry file: {}", entry_file).as_str())
                .to_str()
                .expect(format!("Could not parse directory of entry file: {}", entry_file).as_str())
                .to_string(),
            vec![p
                .file_stem()
                .expect(format!("Could not parse file in of entry file: {}", entry_file).as_str())
                .to_string_lossy()
                .to_string()],
            features,
        );
        let deps = deps
            .iter()
            .map(|d| (vec![d.to_string()], String::new()))
            .collect::<Vec<_>>();

        for f in self.files[n..].iter_mut() {
            f.uses.append(&mut deps.to_vec());
            f.add_prefix(prefix.to_string())
        }
        self.prefixes.push(prefix.to_string());
    }

    // TODO: Doesn't work if a module is declared manually
    fn parse_file(&mut self, dir: String, path: Vec<String>, features: Vec<String>) {
        let dir_name = format!("{}/{}", dir, path.join("/"));
        let dir_p = Path::new(&dir_name);
        // If it is a folder, visit mod.rs
        let (file_name, neighbor) = if dir_p.exists() {
            (format!("{}/mod.rs", dir_name), false)
        // Otherwise visit it as a file
        } else {
            (format!("{}.rs", dir_name), true)
        };
        let file_p = Path::new(&file_name);
        if file_p.exists() {
            let mut file = File::open(file_p).expect("Unable to open file");
            let mut src = String::new();
            file.read_to_string(&mut src).expect("Unable to read file");
            match syn::parse_file(&src) {
                Ok(mut ast) => {
                    let mut vis = AstVisitor::new(features.to_vec());
                    vis.path = path.to_vec();
                    if self.files.is_empty() {
                        vis.is_entry = true;
                        // Add implicit use paths for anything in "crate::"
                        vis.uses.push((
                            concatenate(vec!["crate".to_string()], vec!["*".to_string()]),
                            String::new(),
                        ));
                    } else {
                        // Add implicit use paths derived from using "(super::)+"
                        vis.uses.append(
                            &mut path
                                .iter()
                                .enumerate()
                                .map(|(i, _)| {
                                    (
                                        concatenate(
                                            vec!["crate".to_string()],
                                            path[..i + 1].to_vec(),
                                        ),
                                        String::new(),
                                    )
                                })
                                .collect(),
                        );
                        // Add implicit use paths for anything in "crate::path::to::module::"
                        vis.uses.push((
                            concatenate(
                                concatenate(vec!["crate".to_string()], path.to_vec()),
                                vec!["*".to_string()],
                            ),
                            String::new(),
                        ));
                    }
                    syn::visit_mut::visit_file_mut(&mut vis, &mut ast);
                    let mods = vis.modules.to_vec();
                    self.files.push(vis);
                    for mod_name in mods.iter() {
                        // If it is a neighbor, replace the last element of path
                        let mut new_path = if neighbor {
                            path[1..].to_vec()
                        // Otherwise just extend the path
                        } else {
                            path.to_vec()
                        };
                        new_path.push(mod_name.to_string());
                        self.parse_file(dir.to_string(), new_path, features.to_vec())
                    }
                }
                Err(e) => eprintln!("Failed: {}", e),
            }
        }
    }

    pub fn get_data(&self) -> AstData {
        // Get components and events
        let (components, mut globals, events) = self.files.iter().fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut cs, mut gs, mut es), vis| {
                cs.append(&mut vis.components.to_vec());
                gs.append(&mut vis.globals.to_vec());
                es.append(&mut vis.events.to_vec());
                (cs, gs, es)
            },
        );
        let traits = self.prefixes.iter().fold(Vec::new(), |mut v, pre| {
            for (tr, gl) in [
                (COMPONENTS_TRAIT, COMPONENTS_MANAGER),
                (EVENTS_TRAIT, EVENTS_MANAGER),
            ] {
                v.push(Trait {
                    g_trait: Global {
                        path: vec![pre.to_string(), tr.to_string()],
                        args: GlobalMacroArgs::from(Vec::new()),
                    },
                    global: Global {
                        path: vec![pre.to_string(), gl.to_string()],
                        args: GlobalMacroArgs::from(Vec::new()),
                    },
                });
            }
            v
        });

        // Get systems and map args to components
        let (systems, systems_data) =
            self.files
                .iter()
                .fold((Vec::new(), Vec::new()), |(mut ss, mut data), vis| {
                    ss.append(&mut vis.systems.to_vec());
                    if !vis.systems.is_empty() {
                        data.push(to_data(&vis.systems, |s| {
                            s.to_data(&vis.uses, &components, &globals, &traits, &events)
                        }))
                    }
                    (ss, data)
                });
        let systems_data = systems_data.join(" ");

        // Add traits
        globals.append(&mut traits.into_iter().map(|t| t.global).collect());

        // Get components/event data
        let components_data = to_data(&components, |c| c.to_data());
        let globals_data = to_data(&globals, |g| g.to_data());
        let events_data = to_data(&events, |e| e.to_data());

        AstData {
            components,
            components_data,
            globals,
            globals_data,
            events,
            events_data,
            systems,
            systems_data,
        }
    }
}

impl std::fmt::Display for AstParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for v in self.files.iter() {
            if let Err(e) = f
                .write_fmt(format_args!("File: {}\n", v.to_file()))
                .and_then(|_| f.write_fmt(format_args!("Uses: {}\n", v.uses_string())))
                .and_then(|_| f.write_fmt(format_args!("Components: {}\n", v.components_string())))
                .and_then(|_| f.write_fmt(format_args!("Globals: {}\n", v.globals_string())))
                .and_then(|_| f.write_fmt(format_args!("Systems: {}\n", v.systems_string())))
                .and_then(|_| f.write_fmt(format_args!("Events: {}\n\n", v.events_string())))
            {
                return Err(e);
            }
        }
        Ok(())
    }
}

// Convert parsing to data
pub struct AstData {
    components: Vec<Component>,
    pub components_data: String,
    globals: Vec<Global>,
    pub globals_data: String,
    events: Vec<EventMod>,
    pub events_data: String,
    systems: Vec<System>,
    pub systems_data: String,
}

impl std::fmt::Display for AstData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Components: {}\n", self.components_data))
            .and_then(|_| f.write_fmt(format_args!("Globals: {}\n", self.globals_data)))
            .and_then(|_| f.write_fmt(format_args!("Events: {}\n", self.events_data)))
            .and_then(|_| f.write_fmt(format_args!("Systems: {}\n", self.systems_data)))
    }
}
