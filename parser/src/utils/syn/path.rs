// To path
pub fn vec_to_path(path: Vec<String>) -> syn::Path {
    string_to_path(path.join("::"))
}

pub fn arr_to_path<const N: usize>(path: [&str; N]) -> syn::Path {
    string_to_path(path.join("::"))
}

pub fn string_to_path(path: String) -> syn::Path {
    syn::parse_str(&path).expect(format!("Could not parse path: {}", path).as_str())
}
