use diagnostic::CatchErr;
use proc_macro2::TokenStream;
use shared::syn::{
    error::{SpannedResult, ToError},
    use_path_from_syn,
};
use syn::spanned::Spanned;

use crate::parse::{attributes::get_attributes_if_active, AstMod};

use super::AstItemData;

#[derive(Debug)]
pub struct AstMacroCall {
    pub args: TokenStream,
    pub data: AstItemData,
}

impl AstMod {
    // Macro call
    pub fn visit_item_macro(&mut self, i: syn::ItemMacro) -> SpannedResult<()> {
        if let Some(_) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new())? {
            // Some is for macro_rules!
            if i.ident.is_none() {
                let path = use_path_from_syn(&self.path, &i.mac.path);
                self.items.macro_calls.push(AstMacroCall {
                    data: AstItemData {
                        ident: path
                            .last()
                            .catch_err(i.error("Empty macro path"))?
                            .to_string(),
                        path,
                        span: i.mac.span(),
                    },
                    args: i.mac.tokens,
                });
            }
        }
        Ok(())
    }
}
