use proc_macro2::TokenStream;
use quote::quote;
use shared::util::JoinMap;

use crate::{codegen2::idents::CodegenIdents, resolve::ItemComponent};

pub fn codegen_components(components: &Vec<ItemComponent>) -> TokenStream {
    let struct_name = CodegenIdents::CFoo.to_ident();

    quote!(
        pub struct #struct_name {}
    )
}
