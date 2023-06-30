use shared::util::{Get2D, JoinMap, JoinMapInto, NoneOr};

use crate::parse::AstCrate;
use crate::resolve::constants::NAMESPACE;

#[derive(Debug)]
pub struct CratePaths {
    paths: Vec<Vec<Option<Vec<String>>>>,
}

impl CratePaths {
    pub fn new(crates: &Vec<AstCrate>) -> Self {
        let mut paths = crates.map_vec(|_| Vec::new());
        paths[0] = (0..crates.len()).map_vec(|j| Self::find_path(&mut paths, 0, j, crates));
        Self { paths }
    }

    pub fn get_path(&self, start_idx: usize, end_idx: usize) -> Option<Vec<String>> {
        self.paths.get(start_idx, end_idx).and_then(|v| v.clone())
    }

    pub fn prepend_path(
        &self,
        start_idx: usize,
        end_idx: usize,
        path: &Vec<String>,
    ) -> Option<Vec<String>> {
        let i = if path.starts_with(&["crate".to_string()]) {
            1
        } else {
            0
        };
        self.get_path(start_idx, end_idx)
            .map(|pre| [pre, path[i..].to_vec()].concat())
    }

    fn find_path(
        paths: &mut Vec<Vec<Option<Vec<String>>>>,
        start_idx: usize,
        end_idx: usize,
        crates: &Vec<AstCrate>,
    ) -> Option<Vec<String>> {
        // Base case
        if start_idx == end_idx {
            return Some(vec!["crate".to_string()]);
        }

        let mut min_cr = None;
        for (cr_idx, alias) in &crates[start_idx].deps {
            // We are neighbors with the target
            if *cr_idx == end_idx {
                return Some(vec![alias.to_string()]);
            }

            // Fill paths from neighbor
            if paths[*cr_idx].is_empty() {
                paths[*cr_idx] =
                    (0..crates.len()).map_vec(|j| Self::find_path(paths, *cr_idx, j, crates))
            }

            // Check if shortest path
            if let Some(Some(path)) = paths.get(*cr_idx, end_idx) {
                let path = [[alias, NAMESPACE].map_vec(|s| s.to_string()), path.to_vec()].concat();
                let len = path.iter().fold(0, |s, path| s + path.len());
                if min_cr.is_none_or(|(curr_len, _)| &len < curr_len) {
                    min_cr = Some((len, path));
                }
            }
        }

        min_cr.map(|(_, path)| path)
    }
}
