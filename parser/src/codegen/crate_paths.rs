use diagnostic::{CombineResults, ToErr};
use shared::{
    syn::{
        error::{GetVec, Result, StrToError},
        vec_to_path,
    },
    traits::{AndThen, CollectVec, CollectVecInto, ExpandEnum, Get2D, NoneOr},
};

use crate::{
    parse::{AstCrate, AstMod, ItemPath, ItemSpan},
    utils::{
        constants::NAMESPACE,
        paths::{Crate, CratePath},
    },
};

#[derive(Debug)]
pub struct Crates {
    paths: Vec<Vec<Option<Vec<String>>>>,
    crate_idxs: [usize; Crate::LEN],
    crates: Vec<AstCrate>,
}

impl Crates {
    pub fn new(crates: Vec<AstCrate>, crate_idxs: [usize; Crate::LEN]) -> Result<Self> {
        let mut paths = crates.map_vec(|_| Vec::new());
        // Start with main crate
        *paths.try_get_mut(crate_idxs[Crate::Main as usize])? = (0..crates.len())
            .map_vec_into(|j| Self::find_path(&mut paths, 0, j, &crates))
            .combine_results()?;
        Ok(Self {
            paths,
            crate_idxs,
            crates,
        })
    }

    fn find_path(
        paths: &mut Vec<Vec<Option<Vec<String>>>>,
        start_idx: usize,
        end_idx: usize,
        crates: &Vec<AstCrate>,
    ) -> Result<Option<Vec<String>>> {
        // Base case
        if start_idx == end_idx {
            return Ok(Some(vec!["crate".to_string()]));
        }

        let mut min_cr = None;
        for (cr_idx, alias) in &crates.try_get(start_idx)?.deps {
            // We are neighbors with the target
            if *cr_idx == end_idx {
                return Ok(Some(vec![alias.to_string()]));
            }

            // Fill paths from neighbor
            if paths.try_get(*cr_idx)?.is_empty() {
                *paths.try_get_mut(*cr_idx)? = (0..crates.len())
                    .map_vec_into(|j| Self::find_path(paths, *cr_idx, j, crates))
                    .combine_results()?;
            }

            // Check if shortest path
            if let Some(Some(path)) = paths.get2d(*cr_idx, end_idx) {
                let path = [[alias, NAMESPACE].map_vec(|s| s.to_string()), path.to_vec()].concat();
                let len = path.iter().fold(0, |s, path| s + path.len());
                if min_cr.is_none_or(|(curr_len, _)| &len < curr_len) {
                    min_cr = Some((len, path));
                }
            }
        }

        Ok(min_cr.map(|(_, path)| path))
    }

    pub fn get_crate_paths<const N: usize>(
        &self,
        cr_idx: usize,
        block_crates: [usize; N],
    ) -> Result<Vec<(usize, Vec<String>)>> {
        self.paths.try_get(cr_idx).map(|v| {
            v.enumer_filter_map_vec(|(i, path)| {
                (!block_crates.contains(&i))
                    .and_then(|| path.as_ref().map(|path| (i, path.to_vec())))
            })
        })
    }

    pub fn get_crate_syn_paths<const N: usize>(
        &self,
        cr_idx: usize,
        block_crates: [usize; N],
    ) -> Result<Vec<(usize, syn::Path)>> {
        self.get_crate_paths(cr_idx, block_crates)
            .and_then(|paths| {
                paths
                    .map_vec_into(|(i, path)| vec_to_path(path).map(|path| (i, path)))
                    .combine_results()
            })
    }

    pub fn has_errors(&self) -> bool {
        for cr in &self.crates {
            for m in cr.iter_mods() {
                if !m.errs.is_empty() {
                    return true;
                }
            }
        }
        false
    }

    // Create crate paths
    pub fn get_crate_path(&self, start_idx: usize, end_idx: usize) -> Option<Vec<String>> {
        self.paths.get2d(start_idx, end_idx).and_then(|v| v.clone())
    }

    pub fn get_crate_syn_path(&self, start_idx: usize, end_idx: usize) -> Result<syn::Path> {
        self.get_crate_path(start_idx, end_idx)
            .ok_or(
                format!("No path from {start_idx} to {end_idx}")
                    .trace()
                    .as_vec(),
            )
            .and_then(|v| vec_to_path(v))
    }

    pub fn get_named_crate_path(&self, start_idx: usize, cr: Crate) -> Option<Vec<String>> {
        self.get_crate_path(start_idx, self.get_crate_index(cr))
    }

    pub fn get_named_crate_syn_path(&self, start_idx: usize, cr: Crate) -> Result<syn::Path> {
        self.get_named_crate_path(start_idx, cr)
            .ok_or(
                format!("No path from {start_idx} to {cr:#?} crate")
                    .trace()
                    .as_vec(),
            )
            .and_then(|v| vec_to_path(v))
    }

    pub fn get_item_path(&self, start_idx: usize, path: &ItemPath) -> Result<Vec<String>> {
        let i = if path.path.starts_with(&["crate".to_string()]) {
            1
        } else {
            0
        };
        self.get_crate_path(start_idx, path.cr_idx).map_or(
            format!("No path from crate {start_idx} to crate {}", path.cr_idx)
                .trace()
                .as_err(),
            |pre| Ok([pre, path.path[i..].to_vec()].concat()),
        )
    }

    pub fn get_path(&self, start_idx: usize, item: &CratePath) -> Result<Vec<String>> {
        self.get_item_path(
            start_idx,
            &ItemPath::new(self.get_crate_index(item.cr), item.full_path()),
        )
    }

    pub fn get_item_syn_path(&self, start_idx: usize, path: &ItemPath) -> Result<syn::Path> {
        self.get_item_path(start_idx, path)
            .and_then(|v| vec_to_path(v))
    }

    pub fn get_syn_path(&self, start_idx: usize, item: &CratePath) -> Result<syn::Path> {
        self.get_path(start_idx, item).and_then(|v| vec_to_path(v))
    }

    // Get crates
    pub fn get_crates<'a>(&'a self) -> &'a Vec<AstCrate> {
        &self.crates
    }

    pub fn get_crate<'a>(&'a self, cr: Crate) -> Result<&'a AstCrate> {
        self.crates.try_get(self.get_crate_index(cr))
    }

    pub fn get_crate_mut<'a>(&'a mut self, cr: Crate) -> Result<&'a mut AstCrate> {
        self.crates.try_get_mut(self.get_crate_index(cr))
    }

    pub fn get_crate_index(&self, cr: Crate) -> usize {
        self.crate_idxs[cr as usize]
    }

    pub fn get_mod<'a>(&'a self, cr_idx: usize, mod_idx: usize) -> Result<&'a AstMod> {
        self.crates
            .try_get(cr_idx)
            .and_then(|cr| cr.mods.try_get(mod_idx))
    }

    pub fn get_mod_mut<'a>(&'a mut self, cr_idx: usize, mod_idx: usize) -> Result<&'a mut AstMod> {
        self.crates
            .try_get_mut(cr_idx)
            .and_then(|cr| cr.mods.try_get_mut(mod_idx))
    }

    // Iterate crates except macros crate
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

impl GetVec<AstCrate> for Crates {
    fn try_get<'a>(&'a self, i: usize) -> Result<&'a AstCrate> {
        self.crates.try_get(i)
    }

    fn try_get_mut<'a>(&'a mut self, i: usize) -> Result<&'a mut AstCrate> {
        self.crates.try_get_mut(i)
    }
}
