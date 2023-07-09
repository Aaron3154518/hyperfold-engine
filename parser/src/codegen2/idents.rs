use quote::format_ident;

use crate::{
    resolve::constants::NAMESPACE,
    resolve::{EngineGlobals, EngineTraits, ExpandEnum, GetPaths, NamespaceTraits},
};

pub enum CodegenIdents {
    // General
    Namespace,
    // Systems
    SFooType,
    // Globals
    GFooType,
    // Components
    CFooType,
    AddComponent,
    // Events
    EFooType,
    AddEvent,
    E,
    ELen,
    // Code generation
    GenE,
    GenV,
    GenEid,
    GenEids,
    CFooVar,
    GFooVar,
    EFooVar,
}

impl CodegenIdents {
    pub fn as_str(&self) -> &str {
        match self {
            CodegenIdents::Namespace => NAMESPACE,
            CodegenIdents::SFooType => "SFoo",
            CodegenIdents::GFooType => "GFoo",
            CodegenIdents::CFooType => EngineGlobals::CFoo.get_ident(),
            CodegenIdents::AddComponent => NamespaceTraits::AddComponent.get_ident(),
            CodegenIdents::EFooType => EngineGlobals::EFoo.get_ident(),
            CodegenIdents::AddEvent => NamespaceTraits::AddEvent.get_ident(),
            CodegenIdents::E => "E",
            CodegenIdents::ELen => "E_LEN",
            CodegenIdents::GenE => "e",
            CodegenIdents::GenV => "v",
            CodegenIdents::GenEid => "eid",
            CodegenIdents::GenEids => "eids",
            CodegenIdents::CFooVar => "cfoo",
            CodegenIdents::GFooVar => "gfoo",
            CodegenIdents::EFooVar => "efoo",
        }
    }

    pub fn to_ident(&self) -> syn::Ident {
        format_ident!("{}", self.as_str())
    }
}
