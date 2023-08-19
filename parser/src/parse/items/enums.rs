use shared::{syn::error::Result, traits::PushInto};
use syn::spanned::Spanned;

use crate::parse::{attributes::get_attributes_if_active, AstAttribute, AstMod};

use super::AstItemData;

#[derive(Debug)]
pub struct AstEnum {
    pub attrs: Vec<AstAttribute>,
    pub data: AstItemData,
}

impl AstMod {
    pub fn visit_item_enum(&mut self, i: syn::ItemEnum) -> Result<()> {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new())? {
            if !attrs.is_empty() {
                self.items.enums.push(AstEnum {
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
