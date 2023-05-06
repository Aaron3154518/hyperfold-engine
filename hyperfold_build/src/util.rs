use parse_cfg::Cfg;
use quote::ToTokens;

// Trait for methods that need a path
pub trait HasPath {
    fn get_path_str(&self) -> String;
}

// Comma-delimited list of paths
pub fn join_paths<T>(v: &Vec<T>) -> String
where
    T: HasPath,
{
    v.iter()
        .map(|t| t.get_path_str())
        .collect::<Vec<_>>()
        .join(", ")
}

// Simple concatenate method for two vectors
pub fn concatenate<T: Clone>(mut v1: Vec<T>, mut v2: Vec<T>) -> Vec<T> {
    v1.append(&mut v2);
    v1
}

// Get nth index from the end (n = 0 gives index of last element)
pub fn end<T>(v: &Vec<T>, n: usize) -> usize {
    if v.len() <= n {
        0
    } else {
        v.len() - n - 1
    }
}

// Parse attributes from engine components
#[derive(Copy, Clone, Debug)]
pub enum EcsAttribute {
    Component,
    Global,
    System,
    Event,
}

#[derive(Clone, Debug)]
pub enum Attribute {
    Ecs(EcsAttribute, Vec<String>),
    Cfg(Cfg),
}

pub fn get_attributes_if_active(
    attrs: &Vec<syn::Attribute>,
    features: &Vec<String>,
) -> Option<Vec<(EcsAttribute, Vec<String>)>> {
    let mut is_active = true;
    let new_attrs = get_attributes(attrs)
        .into_iter()
        .fold(Vec::new(), |mut new_attrs, a| {
            match a {
                Attribute::Ecs(ty, args) => new_attrs.push((ty, args)),
                Attribute::Cfg(cfg) => {
                    is_active = !eval_cfg_args(&cfg, features).is_some_and(|b| !b)
                }
            }
            new_attrs
        });
    if is_active {
        Some(new_attrs)
    } else {
        None
    }
}

// TODO: validate path
// Returns list of parsed attributes from ast attributes
pub fn get_attributes(attrs: &Vec<syn::Attribute>) -> Vec<Attribute> {
    attrs
        .iter()
        .filter_map(|a| {
            a.path()
                .segments
                .last()
                .and_then(|s| match s.ident.to_string().as_str() {
                    "component" => Some(Attribute::Ecs(EcsAttribute::Component, Vec::new())),
                    "global" => Some(Attribute::Ecs(EcsAttribute::Global, Vec::new())),
                    "system" => Some(Attribute::Ecs(EcsAttribute::System, Vec::new())),
                    "event" => Some(Attribute::Ecs(EcsAttribute::Event, Vec::new())),
                    "cfg" => Some(Attribute::Cfg(Cfg::Is(String::new()))),
                    _ => None,
                })
                .map(|attr| parse_attr_args(attr, a))
        })
        .collect()
}

pub fn eval_cfg_args(cfg: &Cfg, features: &Vec<String>) -> Option<bool> {
    match cfg {
        Cfg::Any(cfgs) => Some(
            cfgs.iter()
                .map(|cfg| eval_cfg_args(&cfg, &features).is_some_and(|b| b))
                .collect::<Vec<_>>()
                .contains(&true),
        ),
        Cfg::All(cfgs) => Some(
            !cfgs
                .iter()
                .map(|cfg| !eval_cfg_args(&cfg, &features).is_some_and(|b| !b))
                .collect::<Vec<_>>()
                .contains(&false),
        ),
        Cfg::Not(cfg) => eval_cfg_args(cfg, &features).map(|b| !b),
        Cfg::Equal(k, v) => {
            if k == "feature" {
                Some(features.contains(&v))
            } else {
                None
            }
        }
        Cfg::Is(_) => None,
    }
}

// Parses arguments to a single ast attribute
fn parse_attr_args(mut attr_type: Attribute, attr: &syn::Attribute) -> Attribute {
    match &mut attr_type {
        Attribute::Ecs(_, v) => match &attr.meta {
            syn::Meta::List(l) => {
                for t in l.to_token_stream() {
                    match t {
                        proc_macro2::TokenTree::Group(g) => {
                            *v = g
                                .stream()
                                .into_iter()
                                .filter_map(|tt| match tt {
                                    proc_macro2::TokenTree::Ident(i) => Some(i.to_string()),
                                    _ => None,
                                })
                                .collect();
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        },
        Attribute::Cfg(cfg) => match &attr.meta {
            syn::Meta::List(l) => {
                *cfg = l
                    .to_token_stream()
                    .to_string()
                    .parse()
                    .expect("Could not parse cfg_str");
            }
            _ => (),
        },
    };
    attr_type
}

// Get all possible paths to `path` give the set of use paths/aliases
pub fn get_possible_use_paths(
    path: &Vec<String>,
    use_paths: &Vec<(Vec<String>, String)>,
) -> Vec<Vec<String>> {
    use_paths
        .iter()
        .map(|(u_path, alias)| match (u_path.last(), path.first()) {
            (Some(u), Some(t)) => {
                if u == "*" {
                    concatenate(u_path[..end(&u_path, 0)].to_vec(), path.to_vec())
                } else if t == u {
                    if alias.is_empty() {
                        concatenate(u_path.to_vec(), path[1..].to_vec())
                    } else {
                        concatenate(
                            u_path[..end(&u_path, 0)].to_vec(),
                            concatenate(vec![alias.to_string()], path[1..].to_vec()),
                        )
                    }
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        })
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>()
}
