use ecs_macros::structs::LabelType;
use quote::ToTokens;

pub const AND: usize = LabelType::And as usize;
pub const OR: usize = LabelType::Or as usize;
pub const NAND: usize = LabelType::Nand as usize;
pub const NOR: usize = LabelType::Nor as usize;

pub fn type_to_ref_type(ty: &syn::Type, m: bool) -> syn::Type {
    string_to_ref_type(ty.to_token_stream().to_string(), m)
}

pub fn path_to_ref_type(path: Vec<String>, m: bool) -> syn::Type {
    string_to_ref_type(path.join("::"), m)
}

pub fn string_to_ref_type(ty: String, m: bool) -> syn::Type {
    syn::parse_str::<syn::Type>(format!("&{}{}", if m { "mut " } else { "" }, ty).as_str())
        .expect("Could not parse type")
}
