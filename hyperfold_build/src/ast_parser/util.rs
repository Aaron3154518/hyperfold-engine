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
pub enum Attribute {
    Component,
    Global,
    System,
    Event,
}

// TODO: validate path
// Returns list of parsed attributes from ast attributes
pub fn get_attributes<'a>(attrs: &'a Vec<syn::Attribute>) -> Vec<(Attribute, Vec<String>)> {
    attrs
        .iter()
        .filter_map(|a| {
            a.path()
                .segments
                .last()
                .and_then(|s| match s.ident.to_string().as_str() {
                    "component" => Some(Attribute::Component),
                    "global" => Some(Attribute::Global),
                    "system" => Some(Attribute::System),
                    "event" => Some(Attribute::Event),
                    _ => None,
                })
                .map(|attr| (attr, parse_attr_args(a)))
        })
        .collect()
}

// Parses arguments to a single ast attribute
fn parse_attr_args(attr: &syn::Attribute) -> Vec<String> {
    match &attr.meta {
        syn::Meta::List(l) => {
            for t in l.to_token_stream() {
                match t {
                    proc_macro2::TokenTree::Group(g) => {
                        return g
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
    }
    Vec::new()
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
