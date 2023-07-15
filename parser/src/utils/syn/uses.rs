use shared::traits::{End, GetSlice};

// Manage use statements
pub fn add_use_item(parent_path: &Vec<String>, path: &mut Vec<String>, item: String) {
    match item.as_str() {
        "super" => {
            if path.is_empty() {
                *path = parent_path.slice_to(-1).to_vec();
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

pub fn use_path_from_vec(parent_path: &Vec<String>, path: &Vec<String>) -> Vec<String> {
    let mut res_path: Vec<String> = Vec::new();
    for p in path {
        add_use_item(parent_path, &mut res_path, p.to_string())
    }
    res_path
}

pub fn use_path_from_syn(parent_path: &Vec<String>, path: &syn::Path) -> Vec<String> {
    use_path_from_vec(
        parent_path,
        &path.segments.iter().map(|s| s.ident.to_string()).collect(),
    )
}
