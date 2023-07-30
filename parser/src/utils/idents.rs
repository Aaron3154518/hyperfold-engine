use once_cell::sync::Lazy;

use quote::format_ident;

use super::paths::{ENGINE_GLOBALS, ENGINE_TRAITS};

use super::constants::NAMESPACE;

macro_rules! idents {
    ($const: ident = $ty: ident { $($var: ident => $val: expr),*$(,)? }) => {
        pub struct $ty {
            $(pub $var: syn::Ident),*
        }

        impl $ty {
            pub fn new() -> Self {
                Self {
                    $($var: format_ident!("{}", $val)),*
                }
            }
        }

        unsafe impl Sync for $ty {}
        unsafe impl Send for $ty {}

        pub static $const: Lazy<$ty> = Lazy::new(|| $ty::new());
    };
}

idents!(CODEGEN_IDENTS = CodegenIdents {
    namespace => NAMESPACE,
    manager => "SFoo",
    globals => "GFoo",
    components => ENGINE_GLOBALS.c_foo.get_ident(),
    events => ENGINE_GLOBALS.e_foo.get_ident(),
    add_component => ENGINE_TRAITS.components.get_ident(),
    add_event => ENGINE_TRAITS.events.get_ident(),
    event_enum => "E",
    event_enum_len => "E_LEN",
    e_var => "e",
    v_var => "v",
    eid_var => "eid",
    eids_var => "eids",
    stack_var => "stack",
    services_var => "services",
    comps_var => "cfoo",
    globals_var => "gfoo",
    events_var => "efoo",
});

// Returns names of codegen items
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

pub fn component_set_keys_fn(cs_idx: usize, ret_vec: bool) -> syn::Ident {
    format_ident!(
        "get_{}_keys{}",
        component_set_var(cs_idx),
        if ret_vec { "_vec" } else { "" }
    )
}

pub fn state_var(s_idx: usize) -> syn::Ident {
    format_ident!("s{s_idx}")
}
