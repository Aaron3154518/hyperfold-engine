use proc_macro2::{Span, TokenStream};
use shared::{syn::error::SpannedResult, traits::PushInto};
use syn::spanned::Spanned;

use crate::parse::{attributes::get_attributes_if_active, AstAttribute, AstMod};

use super::AstItemData;

#[derive(Debug)]
pub struct AstStruct {
    pub attrs: Vec<AstAttribute>,
    pub data: AstItemData,
}

impl AstMod {
    pub fn visit_item_struct(&mut self, i: syn::ItemStruct) -> SpannedResult<()> {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new())? {
            if !attrs.is_empty() {
                self.items.structs.push(AstStruct {
                    attrs,
                    data: AstItemData {
                        path: self.path.to_vec().push_into(i.ident.to_string()),
                        ident: i.ident.to_string(),
                        span: i.span(),
                    },
                });
            }
        }
        Ok(())
    }
}
