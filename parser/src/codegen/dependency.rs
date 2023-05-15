#[derive(Clone, Debug)]
pub struct Dependency {
    pub cr_idx: usize,
    pub alias: String,
}

pub fn get_deps_post_order(deps: &Vec<Vec<Dependency>>) -> Vec<usize> {
    let mut v = Vec::new();
    get_deps_post_impl(&mut v, 0, deps);
    v
}

fn get_deps_post_impl(list: &mut Vec<usize>, cr_idx: usize, deps: &Vec<Vec<Dependency>>) {
    for d in deps[cr_idx].iter() {
        if !list.contains(&d.cr_idx) {
            get_deps_post_impl(list, d.cr_idx, deps);
        }
    }
    list.push(cr_idx);
}
