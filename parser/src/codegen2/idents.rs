use quote::format_ident;

use crate::{
    resolve::constants::NAMESPACE,
    resolve::{EngineGlobals, EngineTraits, ExpandEnum, GetPaths, NamespaceTraits},
};

pub enum CodegenIdents {
    // General
    Namespace,
    // Systems
    SFoo,
    // Globals
    GFoo,
    // Components
    CFoo,
    AddComponent,
    // Events
    EFoo,
    AddEvent,
    E,
    ELen,
    // Code generation
    GenE,
    GenV,
    GenEid,
    GenEids,
    GenCFoo,
    GenGFoo,
    GenEFoo,
}

impl CodegenIdents {
    pub fn as_str(&self) -> &str {
        match self {
            CodegenIdents::Namespace => NAMESPACE,
            CodegenIdents::SFoo => "SFoo",
            CodegenIdents::GFoo => "GFoo",
            CodegenIdents::CFoo => EngineGlobals::CFoo.as_ident(),
            CodegenIdents::AddComponent => NamespaceTraits::AddComponent.as_ident(),
            CodegenIdents::EFoo => EngineGlobals::EFoo.as_ident(),
            CodegenIdents::AddEvent => NamespaceTraits::AddEvent.as_ident(),
            CodegenIdents::E => "E",
            CodegenIdents::ELen => "E_LEN",
            CodegenIdents::GenE => "e",
            CodegenIdents::GenV => "v",
            CodegenIdents::GenEid => "eid",
            CodegenIdents::GenEids => "eids",
            CodegenIdents::GenCFoo => "cfoo",
            CodegenIdents::GenGFoo => "gfoo",
            CodegenIdents::GenEFoo => "efoo",
        }
    }

    pub fn to_ident(&self) -> syn::Ident {
        format_ident!("{}", self.as_str())
    }
}
