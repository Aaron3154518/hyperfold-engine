use std::path::PathBuf;

use crate::{
    codegen::mods::add_traits,
    parse::{
        ast_crate::Crate,
        ast_fn_arg::{FnArg, FnArgType},
        ast_mod::{MarkType, Mod},
    },
    resolve::ast_resolve::resolve_path,
    util::end,
};
use proc_macro2::{token_stream::IntoIter, TokenStream, TokenTree};
use shared::util::{Catch, Get, JoinMap};

use shared::parse_args::{ComponentMacroArgs, GlobalMacroArgs, SystemMacroArgs};

use super::{
    ast_paths::{MacroPaths, Paths},
    ast_resolve::Path,
};

#[derive(Debug)]
pub struct Component {
    pub path: Path,
    pub args: ComponentMacroArgs,
}

#[derive(Clone, Debug)]
pub struct Global {
    pub path: Path,
    pub args: GlobalMacroArgs,
}

#[derive(Clone, Debug)]
pub struct Trait {
    pub path: Path,
    pub g_idx: usize,
}

#[derive(Debug)]
pub struct Event {
    pub path: Path,
}

#[derive(Debug)]
pub struct System {
    pub path: Path,
    pub args: Vec<FnArg>,
    pub attr_args: SystemMacroArgs,
}

#[derive(Debug)]
pub struct Dependency {
    pub cr_idx: usize,
    pub cr_alias: String,
}

#[derive(Debug)]
pub struct ComponentSetItem {
    var: String,
    ty: Vec<String>,
    ref_cnt: usize,
    is_mut: bool,
}

#[derive(Debug)]
pub enum LabelOp {
    And { lhs: LabelItem, rhs: LabelItem },
    Or { lhs: LabelItem, rhs: LabelItem },
    None(LabelItem),
}

// TODO: use fancy syn parsing, not manual
// TODO: Parse types, not idents
impl LabelOp {
    fn parse(lhs_not: bool, lhs_tt: TokenTree, op: TokenTree, rhs: IntoIter) -> Self {
        let lhs = match lhs_tt {
            TokenTree::Group(g) => LabelItem::parse(g),
            TokenTree::Ident(i) => LabelItem::Item { not: lhs_not, ty: vec![] },
            _ => panic!("Expected parentheses or item")
        }

        return match (op, rhs.next()) {
            (TokenTree::Punct(p1), Some(TokenTree::Punct(p2))) => {
                match (p1.as_char(), p2.as_char()) {
                    ('&', '&') => Self::And { lhs: (), rhs: () },
                    ('|', '|') => Self::Or { lhs: (), rhs: () },
                    _ => panic!("Expected '&&' or '||'"),
                }
            }
            _ => panic!("Expected '&&' or '||'"),
        };
    }
}

#[derive(Debug)]
pub enum LabelItem {
    Parens { not: bool, body: Box<LabelOp> },
    Item { not: bool, ty: Vec<String> },
}

impl LabelItem {
    fn parse(g: proc_macro2::Group) -> Self {
        let ts = g.stream();

        let mut it = ts.into_iter();
        match it.next() {
            Some(mut first_tt) => {
                let mut not = false;
                while let TokenTree::Punct(p) = first_tt {
                    assert_eq!(p.as_char(), '!', "Expected '!'");
                    not = !not;
                    first_tt = it.next().catch(format!("Expected NonOp"));
                }
                match it.next() {
                    Some(tt) => {
                        return Self::Parens {
                            not: false,
                            body: Box::new(LabelOp::parse(not, first_tt, tt, it)),
                        }
                    }
                    None => {
                        return Self::Parens {
                            not,
                            body: Box::new(LabelOp::None(LabelItem::parse(match first_tt {
                                TokenTree::Group(g) => g,
                                _ => panic!("Expected token group in single None op"),
                            }))),
                        }
                    }
                }
            }
            None => {
                return Self::Item {
                    not: false,
                    ty: Vec::new(),
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ComponentSet {
    pub path: Path,
    pub args: Vec<ComponentSetItem>,
    pub labels: Option<LabelItem>,
}

impl ComponentSet {
    fn parse(cr_idx: usize, path: Vec<String>, ts: TokenStream) -> Self {
        let mut labels = None;
        let mut path = Path { cr_idx, path };

        let mut it = ts.into_iter();
        let mut loc = "at the beginning".to_string();
        // first ident
        match it.next().catch(format!("Expected item {loc}")) {
            TokenTree::Ident(mut i) => {
                // labels
                if i == "labels" {
                    loc = "in labels".to_string();
                    // "()"
                    labels = Some(LabelItem::parse(
                        match it.next().catch(format!("Expected args {loc}")) {
                            TokenTree::Group(g) => g,
                            _ => panic!("Expected args group {loc}"),
                        },
                    ));
                    loc = "after labels".to_string();
                    // ","
                    match it.next().catch(format!("Missing comma {loc}")) {
                        TokenTree::Punct(p) => {
                            assert_eq!(p.as_char(), ',', "Expected comma {loc}")
                        }
                        _ => panic!("Expected comma {loc}"),
                    }
                    // identifier ident
                    i = match it.next().catch(format!("Expected ident {loc}")) {
                        TokenTree::Ident(i) => i,
                        _ => panic!("Expected identified ident {loc}"),
                    }
                }

                // identifier ident
                path.path.push(i.to_string());
            }
            _ => panic!("Expected an ident {loc}"),
        }

        let mut args = Vec::new();
        let mut next_tt = it.next();
        while let Some(mut tt) = next_tt {
            loc = format!("while parsing component {}", args.len());

            // ","
            match tt {
                TokenTree::Punct(p) => assert_eq!(p.as_char(), ',', "Expected comma {loc}"),
                _ => panic!("Expected comma {loc}"),
            };

            // var
            let var = match it.next().catch(format!("Expected parameter name {loc}")) {
                TokenTree::Ident(i) => i.to_string(),
                _ => panic!("Expected parameter name {loc}"),
            };

            // ":"
            match it.next().catch(format!("Expected colon {loc}")) {
                TokenTree::Punct(p) => assert_eq!(p.as_char(), ':', "Expected colon {loc}"),
                _ => panic!("Expected colon {loc}"),
            }

            tt = it.next().catch(format!("Expected type {loc}"));
            let mut ref_cnt = 0;
            // "&"* "'"lifetime?
            while let TokenTree::Punct(p) = &tt {
                match p.as_char() {
                    '&' => {
                        ref_cnt += 1;
                        tt = it.next().catch(format!("Expected type {loc}"));
                    }
                    '\'' => {
                        match it.next().catch(format!("Expected lifetime name {loc}")) {
                            TokenTree::Ident(_) => (),
                            _ => panic!("Expected lifetime name {loc}"),
                        }
                        break;
                    }
                    _ => panic!("Expected reference or lifetime {loc}"),
                }
            }

            // "mut"? type
            let mut is_mut = false;
            let mut ty = Vec::new();
            match it.next().catch(format!("Expected type name or mut {loc}")) {
                TokenTree::Ident(mut i) => {
                    if i == "mut" {
                        is_mut = true;

                        i = match it.next().catch(format!("Expected type after mut {loc}")) {
                            TokenTree::Ident(i) => i,
                            _ => panic!("Expected type after mut {loc}"),
                        }
                    }

                    ty.push(i.to_string());
                }
                _ => panic!("Expected type name or mut {loc}"),
            };

            // "::"type*
            next_tt = it.next();
            while let Some(TokenTree::Punct(p)) = &next_tt {
                match p.as_char() {
                    ':' => {
                        match it.next().catch(format!("Expected ':' in type {loc}")) {
                            TokenTree::Punct(p) => {
                                assert_eq!(p.as_char(), ':', "Expected ':' in type {loc}");
                                ty.push(match it.next().catch(format!("Expected type {loc}")) {
                                    TokenTree::Ident(i) => i.to_string(),
                                    _ => panic!("Expected type {loc}"),
                                });
                            }
                            _ => panic!("Expected ':' in type {loc}"),
                        }
                        next_tt = it.next();
                    }
                    _ => break,
                }
            }

            args.push(ComponentSetItem {
                var,
                ty,
                ref_cnt,
                is_mut,
            })
        }

        Self { path, args, labels }
    }
}

// Pass 2: use resolving
// Resolve macro paths - convert to engine items
#[derive(Debug)]
pub struct ItemsCrate {
    pub dir: PathBuf,
    pub cr_name: String,
    pub cr_idx: usize,
    pub components: Vec<Component>,
    pub globals: Vec<Global>,
    pub traits: Vec<Trait>,
    pub events: Vec<Event>,
    pub systems: Vec<System>,
    pub dependencies: Vec<Dependency>,
    pub component_sets: Vec<ComponentSet>,
}

impl ItemsCrate {
    pub fn new() -> Self {
        Self {
            dir: PathBuf::new(),
            cr_name: String::new(),
            cr_idx: 0,
            components: Vec::new(),
            globals: Vec::new(),
            traits: Vec::new(),
            events: Vec::new(),
            systems: Vec::new(),
            dependencies: Vec::new(),
            component_sets: Vec::new(),
        }
    }

    pub fn parse(paths: &Paths, crates: &Vec<Crate>) -> Vec<Self> {
        // Skip macros crate
        let mut items = crates[..end(&crates, 1)].map_vec(|cr| {
            let mut ic = ItemsCrate::new();
            ic.parse_crate(cr, &paths, &crates);
            // Remove macros crate as crate dependency
            if let Some(i) = ic
                .dependencies
                .iter()
                .position(|d| d.cr_idx == crates.len() - 1)
            {
                ic.dependencies.swap_remove(i);
            }
            ic
        });
        add_traits(&mut items);
        items
    }

    pub fn parse_crate(&mut self, cr: &Crate, paths: &Paths, crates: &Vec<Crate>) {
        self.dir = cr.dir.to_owned();
        self.cr_name = cr.name.to_string();
        self.cr_idx = cr.idx;
        self.dependencies = cr
            .deps
            .iter()
            .map(|(&cr_idx, alias)| Dependency {
                cr_idx,
                cr_alias: alias.to_string(),
            })
            .collect::<Vec<_>>();
        self.parse_mod(cr, &cr.main, paths, crates)
    }

    pub fn parse_mod(&mut self, cr: &Crate, m: &Mod, paths: &Paths, crates: &Vec<Crate>) {
        let cr_idx = cr.idx;

        for mi in m.marked.iter() {
            for (path, args) in mi.attrs.iter() {
                let match_path = resolve_path(path.to_vec(), cr, m, crates).get();
                match &mi.ty {
                    MarkType::Enum | MarkType::Struct => {
                        if &match_path == paths.get_macro(MacroPaths::Component) {
                            self.components.push(Component {
                                path: Path {
                                    cr_idx,
                                    path: mi.sym.path.to_vec(),
                                },
                                args: ComponentMacroArgs::from(args.to_vec()),
                            });
                            break;
                        } else if &match_path == paths.get_macro(MacroPaths::Global) {
                            self.globals.push(Global {
                                path: Path {
                                    cr_idx,
                                    path: mi.sym.path.to_vec(),
                                },
                                args: GlobalMacroArgs::from(args.to_vec()),
                            });
                            break;
                        } else if &match_path == paths.get_macro(MacroPaths::Event) {
                            self.events.push(Event {
                                path: Path {
                                    cr_idx,
                                    path: mi.sym.path.to_vec(),
                                },
                            })
                        }
                    }
                    MarkType::Fn { args: fn_args } => {
                        if &match_path == paths.get_macro(MacroPaths::System) {
                            self.systems.push(System {
                                path: Path {
                                    cr_idx,
                                    path: mi.sym.path.to_vec(),
                                },
                                args: fn_args
                                    .iter()
                                    .map(|a| {
                                        let mut a = a.to_owned();
                                        a.resolve_paths(cr, m, crates);
                                        a
                                    })
                                    .collect(),
                                attr_args: SystemMacroArgs::from(args.to_vec()),
                            });
                            break;
                        }
                    }
                }
            }
        }

        // Todo: needs to happen in separate pass within ast_mod
        // 1: Match "components!"; Parse args into blackbox types; Add export type to mod
        // 2: Do resolution pass; Resolve all parse args + labels
        let components_path = paths.get_macro(MacroPaths::Components);
        for mc in m.macro_calls.iter() {
            if &resolve_path(mc.path.to_vec(), cr, m, crates).get() == components_path {
                eprintln!("{:#?}", mc.args);
                eprintln!(
                    "{:#?}",
                    ComponentSet::parse(cr_idx, m.path.to_vec(), mc.args.clone())
                )
            }
        }

        m.mods
            .iter()
            .for_each(|m| self.parse_mod(cr, m, paths, crates));
    }
}
