use shared::util::{Get2D, JoinMap, JoinMapInto, NoneOr};

use crate::parse::AstCrate;
use crate::resolve::constants::NAMESPACE;

#[derive(Debug)]
pub struct CratePaths {
    paths: Vec<Vec<Option<String>>>,
}

impl CratePaths {
    pub fn new(crates: &Vec<AstCrate>) -> Self {
        let mut paths = crates.map_vec(|_| Vec::new());
        paths[0] = (0..crates.len()).map_vec(|j| Self::find_path(&mut paths, 0, j, crates));
        Self { paths }
    }

    pub fn get_path(&self, start_idx: usize, end_idx: usize) -> Option<String> {
        self.paths
            .get(start_idx, end_idx)
            .and_then(|p| p.as_ref().map(|path| format!("crate{path}")))
    }

    fn find_path(
        paths: &mut Vec<Vec<Option<String>>>,
        start_idx: usize,
        end_idx: usize,
        crates: &Vec<AstCrate>,
    ) -> Option<String> {
        // Base case
        if start_idx == end_idx {
            return Some(String::new());
        }

        let mut min_cr = None;
        for (cr_idx, alias) in &crates[start_idx].deps {
            // We are neighbors with the target
            if *cr_idx == end_idx {
                return Some(format!("::{NAMESPACE}::{alias}"));
            }

            // Fill paths from neighbor
            if paths[*cr_idx].is_empty() {
                paths[*cr_idx] =
                    (0..crates.len()).map_vec(|j| Self::find_path(paths, *cr_idx, j, crates))
            }

            // Check if shortest path
            if let Some(Some(path)) = paths.get(*cr_idx, end_idx) {
                let path = format!("::{NAMESPACE}::{alias}{path}");
                if min_cr.is_none_or(|curr_path: &String| path.len() < curr_path.len()) {
                    min_cr = Some(path);
                }
            }
        }

        min_cr
    }
}
