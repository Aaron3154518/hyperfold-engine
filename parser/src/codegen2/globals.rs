use proc_macro2::TokenStream;
use quote::quote;
use shared::util::JoinMap;

use crate::{codegen2::idents::CodegenIdents, resolve::ItemGlobal};

pub fn globals(globals: &Vec<ItemGlobal>) -> TokenStream {
    let struct_name = CodegenIdents::GFoo.to_ident();

    quote!(
        pub struct #struct_name {}
    )
}
