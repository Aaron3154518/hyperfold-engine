pub mod ast_visitor;
pub mod component;
pub mod event;
pub mod system;
pub mod util;

use ast_visitor::AstVisitor;
use component::{Component, Global};
use event::EventMod;
use std::{fs::File, io::Read, path::Path};
use system::System;
use util::concatenate;

// Parsing file
pub struct AstParser {
    dir: String,
    files: Vec<AstVisitor>,
}

impl AstParser {
    pub fn parse(entry_file: &str) -> Self {
        let p = Path::new(&entry_file);
        if !p.exists() {
            panic!("Could not open entry file for parsing: {}", entry_file);
        }
        let mut parser = AstParser {
            dir: p
                .parent()
                .expect(format!("Could not parse directory of entry file: {}", entry_file).as_str())
                .to_str()
                .expect(format!("Could not parse directory of entry file: {}", entry_file).as_str())
                .to_string(),
            files: Vec::new(),
        };
        parser.parse_file(vec![p
            .file_stem()
            .expect(format!("Could not parse file in of entry file: {}", entry_file).as_str())
            .to_string_lossy()
            .to_string()]);
        parser
    }

    // TODO: Doesn't work if a module is declared manually
    fn parse_file(&mut self, path: Vec<String>) {
        let dir_name = format!("{}/{}", self.dir, path.join("/"));
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
                    let mut vis = AstVisitor::new();
                    if !path.first().is_some_and(|p| p == "main") {
                        vis.path = path.to_vec();
                        // Add implicit use paths derived from using "super::"
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
                        self.parse_file(new_path)
                    }
                }
                Err(e) => eprintln!("Failed: {}", e),
            }
        }
    }

    // Combines vectors of objects from different visitors
    fn get_data<'a, T, F1, F2>(&'a self, get_vec: F1, to_data: F2) -> (Vec<T>, String)
    where
        T: 'a + Clone,
        F1: Fn(&'a AstVisitor) -> &'a Vec<T>,
        F2: Fn(&T, &'a AstVisitor) -> String,
    {
        let (ts, data) =
            self.files
                .iter()
                .fold((Vec::new(), Vec::new()), |(mut ts, mut data), vis| {
                    let v: &Vec<T> = get_vec(vis);
                    ts.append(&mut v.to_vec());
                    data.append(&mut v.iter().map(|t| to_data(t, vis)).collect::<Vec<_>>());
                    (ts, data)
                });
        (ts, data.join(" "))
    }
}

impl std::fmt::Display for AstParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for v in self.files.iter() {
            if let Err(e) = f
                .write_fmt(format_args!("File: {}/{}\n", self.dir, v.to_file()))
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

impl AstData {
    pub fn from(parser: &AstParser) -> Self {
        let (components, components_data) = parser.get_data(|v| &v.components, |c, _| c.to_data());
        let (globals, globals_data) = parser.get_data(|v| &v.globals, |g, _| g.to_data());
        let (events, events_data) = parser.get_data(|v| &v.events, |e, _| e.to_data());
        let (systems, systems_data) = parser.get_data(
            |v| &v.systems,
            |s, v| s.to_data(&v.uses, &components, &globals, &events),
        );
        Self {
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

impl std::fmt::Display for AstData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Components: {}\n", self.components_data))
            .and_then(|_| f.write_fmt(format_args!("Globals: {}\n", self.globals_data)))
            .and_then(|_| f.write_fmt(format_args!("Events: {}\n", self.events_data)))
            .and_then(|_| f.write_fmt(format_args!("Systems: {}\n", self.systems_data)))
    }
}

// Environment variables to use
pub const COMPONENTS: &str = "COMPONENTS";
pub const GLOBALS: &str = "GLOBALS";
pub const EVENTS: &str = "EVENTS";
pub const SYSTEMS: &str = "SYSTEMS";
