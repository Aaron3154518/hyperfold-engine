use proc_macro2::TokenStream;
use quote::quote;

use crate::codegen2::{CodegenIdents, CODEGEN_IDENTS};

pub fn manager_def() -> TokenStream {
    let CodegenIdents {
        manager,
        globals,
        components,
        events,
        event_enum,
        event_enum_len,
        comps_var,
        globals_var,
        events_var,
        ..
    } = &*CODEGEN_IDENTS;

    quote!(
        pub struct #manager {
            #comps_var: #components,
            #globals_var: #globals,
            #events_var: #events,
            stack: Vec<std::collections::VecDeque<(#event_enum, usize)>>,
            services: [Vec<Box<dyn Fn(&mut #components, &mut #globals, &mut #events)>>; #event_enum_len]
        }
    )
}
