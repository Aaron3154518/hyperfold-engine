use quote::format_ident;

pub const NAMESPACE: &str = "_engine";
pub const SEP: &str = ";";
pub const DATA_FILE: &str = "hyperfold_data.txt";
pub const EID: &str = "id";
pub const INDEX: &str = "hyperfold_engine_index.txt";
pub const INDEX_SEP: &str = "\t";

pub fn component_var(c_idx: usize) -> syn::Ident {
    format_ident!("c{c_idx}")
}

pub fn global_var(g_idx: usize) -> syn::Ident {
    format_ident!("g{g_idx}")
}

pub fn event_var(e_idx: usize) -> syn::Ident {
    format_ident!("e{e_idx}")
}

pub fn event_variant(e_idx: usize) -> syn::Ident {
    format_ident!("E{e_idx}")
}

pub fn component_set_var(cs_idx: usize) -> syn::Ident {
    format_ident!("cs{cs_idx}")
}
