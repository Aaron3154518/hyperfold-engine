use crate::{
    resolve::{
        ast_items::{Dependency, ItemsCrate},
        ast_resolve::Path,
    },
    util::NoneOr,
};

use super::constants::NAMESPACE;

impl Path {
    pub fn path_from(&self, cr_idx: usize, crates: &Vec<ItemsCrate>) -> Vec<String> {
        if cr_idx == self.cr_idx {
            return self.path.to_vec();
        }

        let start_dep = Dependency {
            cr_idx: cr_idx,
            cr_alias: String::new(),
        };

        // Dijkstra's with weight = length of crate name
        let (mut frontier, mut visited) = (vec![(vec![&start_dep], 0)], vec![0]);
        loop {
            let mut min_path = None;
            for (path, score) in frontier.iter() {
                let dep = *path.last().expect("Empty path in root_path()");
                if dep.cr_idx == self.cr_idx {
                    let mut use_path = Vec::new();
                    // Don't include entry crate
                    for d in path[1..].iter() {
                        use_path.push(d.cr_alias.to_string());
                        use_path.push(NAMESPACE.to_string());
                    }
                    // Don't include final NAMESPACE
                    use_path.pop();
                    use_path.extend(self.path[1..].to_vec().into_iter());
                    return use_path;
                }
                for d in crates[dep.cr_idx].dependencies.iter() {
                    if !visited.contains(&d.cr_idx)
                        && min_path
                            .is_none_or(|(_, min_score)| *min_score > *score + d.cr_alias.len())
                    {
                        min_path = Some((
                            [path.to_vec(), vec![&d]].concat(),
                            *score + d.cr_alias.len(),
                        ))
                    }
                }
            }
            if let Some((new_path, new_score)) = min_path {
                visited.push(
                    new_path
                        .last()
                        .expect("Empty new_path in root_path()")
                        .cr_idx,
                );
                frontier.push((new_path, new_score));
            } else {
                panic!(
                    "Could not find path from crate {} to path {} in crate {}",
                    cr_idx,
                    self.path.join("::"),
                    self.cr_idx
                );
            }
        }
    }
}
