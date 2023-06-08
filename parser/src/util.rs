use std::{iter::Map, path::PathBuf};

use regex::Regex;
use shared::util::SplitCollect;

#[inline]
pub fn end<T>(v: &Vec<T>, off: usize) -> usize {
    v.len().max(off) - off
}

// Manage use statements
pub fn add_path_item(parent_path: &Vec<String>, path: &mut Vec<String>, item: String) {
    match item.as_str() {
        "super" => {
            if path.is_empty() {
                *path = parent_path[..end(&parent_path, 1)].to_vec();
            } else {
                path.pop();
            }
        }
        "self" => {
            if path.is_empty() {
                *path = parent_path.to_vec();
            }
        }
        _ => path.push(item),
    }
}

pub fn parse_vec_path(parent_path: &Vec<String>, path: &Vec<String>) -> Vec<String> {
    let mut res_path: Vec<String> = Vec::new();
    for p in path {
        add_path_item(parent_path, &mut res_path, p.to_string())
    }
    res_path
}

pub fn parse_syn_path(parent_path: &Vec<String>, path: &syn::Path) -> Vec<String> {
    parse_vec_path(
        parent_path,
        &path.segments.iter().map(|s| s.ident.to_string()).collect(),
    )
}

// Minimal code formatting for token streams
pub fn format_code(s: String) -> String {
    let space_reg_l = Regex::new(r"(^|\w|\)) (:|::|<|>|;|\.|\(|,|&|})")
        .expect("Could not parse left space codegen regex");
    let space_reg_r = Regex::new(r"(::|<|;|\.|\)|&|\{|}) (\w|\(|$)")
        .expect("Could not parse right space codegen regex");
    brackets(
        space_reg_l
            .replace_all(
                space_reg_r
                    .replace_all(s.replace("; ", ";\n").as_str(), "${1}${2}")
                    .to_string()
                    .as_str(),
                "${1}${2}",
            )
            .to_string(),
    )
}

pub fn brackets(s: String) -> String {
    let mut l_is = s.match_indices("{");
    let mut r_is = s.match_indices("}");
    let mut l_i = l_is.next();
    let mut r_i = r_is.next();
    let idx1 = if let Some((i, _)) = l_i { i } else { return s };
    let mut cnt: usize = 0;
    while let Some((r, _)) = r_i {
        if l_i.is_some_and(|(l, _)| l <= r) {
            l_i = l_is.next();
            cnt += 1;
        } else {
            r_i = r_is.next();
            if cnt == 1 {
                let mid = brackets(s[idx1 + 1..r].to_string())
                    .split_collect::<Vec<_>>("\n")
                    .join("\n\t");
                return format!(
                    "{}{{{}}}{}{}",
                    s[..idx1].to_string(),
                    if mid.trim().is_empty() {
                        String::new()
                    } else {
                        format!("\n\t{}\n", mid)
                    },
                    if r_i.is_some_and(|(r2, _)| r2 != r + 1) {
                        "\n"
                    } else {
                        ""
                    },
                    brackets(s[r + 1..].to_string())
                );
            } else if cnt > 0 {
                cnt -= 1;
            }
        }
    }
    s
}
