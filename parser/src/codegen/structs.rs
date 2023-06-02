use proc_macro2::TokenStream;
use shared::parse_args::ComponentMacroArgs;

pub struct Component {
    pub ty: TokenStream,
    pub var: syn::Ident,
    pub args: ComponentMacroArgs,
}
