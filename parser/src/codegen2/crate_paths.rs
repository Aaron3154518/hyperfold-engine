use shared::util::{AndThen, Get2D, JoinMap, JoinMapInto, NoneOr};

use crate::parse::AstCrate;
use crate::resolve::constants::NAMESPACE;
use crate::resolve::util::MsgsResult;
use crate::resolve::{Crate, CratePath, ExpandEnum, ItemPath};

use super::util::vec_to_path;

#[derive(Debug)]
pub struct Crates {
    paths: Vec<Vec<Option<Vec<String>>>>,
    crate_idxs: [usize; Crate::LEN],
    crates: Vec<AstCrate>,
}

impl Crates {
    pub fn new(crates: Vec<AstCrate>, crate_idxs: [usize; Crate::LEN]) -> Self {
        let mut paths = crates.map_vec(|_| Vec::new());
        paths[0] = (0..crates.len()).map_vec_into(|j| Self::find_path(&mut paths, 0, j, &crates));
        Self {
            paths,
            crate_idxs,
            crates,
        }
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
                    (0..crates.len()).map_vec_into(|j| Self::find_path(paths, *cr_idx, j, crates))
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

    pub fn get_crate_paths<const N: usize>(
        &self,
        cr_idx: usize,
        block_crates: [usize; N],
    ) -> Option<Vec<(usize, Vec<String>)>> {
        let macros_cr_idx = self.get_crate_index(Crate::Macros);
        (&self.paths as &[Vec<Option<Vec<String>>>])
            .get(cr_idx)
            .map(|v| {
                v.enumerate_filter_map_vec(|(i, path)| {
                    (!block_crates.contains(&i))
                        .and_then(|| path.as_ref().map(|path| (i, path.to_vec())))
                })
            })
    }

    // Create crate paths
    pub fn get_crate_path(&self, start_idx: usize, end_idx: usize) -> Option<Vec<String>> {
        self.paths.get(start_idx, end_idx).and_then(|v| v.clone())
    }

    pub fn get_crate_syn_path(&self, start_idx: usize, end_idx: usize) -> MsgsResult<syn::Path> {
        self.get_crate_path(start_idx, end_idx)
            .map(|v| vec_to_path(v))
            .ok_or(vec![format!("No path from {start_idx} to {end_idx}")])
    }

    pub fn get_named_crate_path(&self, start_idx: usize, cr: Crate) -> Option<Vec<String>> {
        self.get_crate_path(start_idx, self.get_crate_index(cr))
    }

    pub fn get_named_crate_syn_path(&self, start_idx: usize, cr: Crate) -> MsgsResult<syn::Path> {
        self.get_named_crate_path(start_idx, cr)
            .map(|v| vec_to_path(v))
            .ok_or(vec![format!("No path from {start_idx} to {cr:#?} crate")])
    }

    pub fn get_item_path(&self, start_idx: usize, path: &ItemPath) -> MsgsResult<Vec<String>> {
        let i = if path.path.starts_with(&["crate".to_string()]) {
            1
        } else {
            0
        };
        self.get_crate_path(start_idx, path.cr_idx).map_or(
            Err(vec![format!(
                "No path from crate {start_idx} to crate {}",
                path.cr_idx
            )]),
            |pre| Ok([pre, path.path[i..].to_vec()].concat()),
        )
    }

    pub fn get_path(&self, start_idx: usize, item: &CratePath) -> MsgsResult<Vec<String>> {
        self.get_item_path(
            start_idx,
            &ItemPath {
                cr_idx: self.get_crate_index(item.cr),
                path: item.full_path(),
            },
        )
    }

    pub fn get_item_syn_path(&self, start_idx: usize, path: &ItemPath) -> MsgsResult<syn::Path> {
        self.get_item_path(start_idx, path).map(|v| vec_to_path(v))
    }

    pub fn get_syn_path(&self, start_idx: usize, item: &CratePath) -> MsgsResult<syn::Path> {
        self.get_path(start_idx, item).map(|v| vec_to_path(v))
    }

    // Get crates
    pub fn get_crates<'a>(&'a self) -> &'a Vec<AstCrate> {
        &self.crates
    }

    pub fn get_crate<'a>(&'a self, cr: Crate) -> &'a AstCrate {
        &self.crates[self.get_crate_index(cr)]
    }

    pub fn get_crate_mut<'a>(&'a mut self, cr: Crate) -> &'a mut AstCrate {
        let i = self.get_crate_index(cr);
        &mut self.crates[i]
    }

    pub fn get_crate_index(&self, cr: Crate) -> usize {
        self.crate_idxs[cr as usize]
    }

    // Iterate crates except macros crate
    pub fn get<'a>(&'a self, cr_idx: usize) -> Option<&'a AstCrate> {
        self.crates.get(cr_idx)
    }

    pub fn get_mut<'a>(&'a mut self, cr_idx: usize) -> Option<&'a mut AstCrate> {
        self.crates.get_mut(cr_idx)
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a AstCrate> {
        self.crates.iter()
    }

    pub fn iter_except<'a, const N: usize>(
        &'a self,
        block_crates: [usize; N],
    ) -> impl Iterator<Item = &'a AstCrate> {
        self.crates
            .iter()
            .enumerate()
            .filter(move |(i, _)| !block_crates.contains(i))
            .map(|(_, v)| v)
    }

    pub fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut AstCrate> {
        self.crates.iter_mut()
    }

    pub fn iter_except_mut<'a, const N: usize>(
        &'a mut self,
        block_crates: [usize; N],
    ) -> impl Iterator<Item = &'a mut AstCrate> {
        self.crates
            .iter_mut()
            .enumerate()
            .filter(move |(i, _)| !block_crates.contains(i))
            .map(|(_, v)| v)
    }
}
