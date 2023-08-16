use shared::{syn::error::SpannedResult, traits::PushInto};
use syn::spanned::Spanned;

use crate::parse::{attributes::get_attributes_if_active, AstAttribute, AstMod};

use super::AstItemData;

#[derive(Debug)]
pub struct AstFunction {
    pub sig: syn::Signature,
    pub attrs: Vec<AstAttribute>,
    pub data: AstItemData,
}

impl AstMod {
    // Systems
    pub fn visit_item_fn(&mut self, i: syn::ItemFn) -> SpannedResult<()> {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new())? {
            if !attrs.is_empty() {
                self.items.functions.push(AstFunction {
                    sig: i.sig,
                    attrs,
                    data: AstItemData {
                        path: self.path.to_vec().push_into(i.sig.ident.to_string()),
                        ident: i.sig.ident.to_string(),
                        span: i.span(),
                    },
                });
            }
        }
        Ok(())
    }
}
