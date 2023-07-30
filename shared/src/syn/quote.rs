use proc_macro2::TokenStream;
use quote::quote;

// Convert ident to TokenStream
pub trait Quote {
    fn quote(&self) -> TokenStream;
}

impl Quote for syn::Ident {
    fn quote(&self) -> TokenStream {
        quote!(#self)
    }
}
