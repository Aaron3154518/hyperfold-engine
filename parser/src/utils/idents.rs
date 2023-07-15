use once_cell::sync::Lazy;

use quote::format_ident;

use crate::{
    resolve::constants::NAMESPACE,
    resolve::{ExpandEnum, ENGINE_GLOBALS, ENGINE_TRAITS},
};

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
    add_component => ENGINE_TRAITS.main_add_component.get_ident(),
    add_event => ENGINE_TRAITS.main_add_event.get_ident(),
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
