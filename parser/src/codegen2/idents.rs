use once_cell::sync::Lazy;

use quote::format_ident;

use crate::{
    resolve::constants::NAMESPACE,
    resolve::{EngineGlobals, EngineTraits, ExpandEnum, GetPaths, NamespaceTraits},
};

macro_rules! codegen_idents {
    ($ty: ident { $($var: ident => $val: expr),*$(,)? }) => {
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
    };
}

codegen_idents!(CodegenIdents {
    namespace => NAMESPACE,
    manager => "SFoo",
    globals => "GFoo",
    components => EngineGlobals::CFoo.get_ident(),
    events => EngineGlobals::EFoo.get_ident(),
    add_component => NamespaceTraits::AddComponent.get_ident(),
    add_event => NamespaceTraits::AddEvent.get_ident(),
    event_enum => "E",
    event_enum_len => "E_LEN",
    e_var => "e",
    v_var => "v",
    eid_var => "eid",
    eids_var => "eids",
    comps_var => "cfoo",
    globals_var => "gfoo",
    events_var => "efoo",
});

unsafe impl Sync for CodegenIdents {}
unsafe impl Send for CodegenIdents {}

pub static CODEGEN_IDENTS: Lazy<CodegenIdents> = Lazy::new(|| CodegenIdents::new());
