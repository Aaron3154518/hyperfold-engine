use proc_macro2::TokenStream;
use shared::util::JoinMap;

use crate::parse::{ast_crate::AstCrate, ast_mod::AstMod};

use super::path::{resolve_path, ItemPath};

pub trait ParseMacroCall
where
    Self: Sized,
{
    fn parse(
        cr: &AstCrate,
        m: &AstMod,
        crates: &Vec<AstCrate>,
        ts: TokenStream,
    ) -> syn::Result<Self>;

    fn update_mod(&self, m: &mut AstMod);
}

pub struct MacroCalls<T> {
    calls: Vec<T>,
    mods: Vec<MacroCalls<T>>,
}

pub fn parse_macro_calls<T>(
    macro_path: &ItemPath,
    m: &AstMod,
    cr: &AstCrate,
    crates: &Vec<AstCrate>,
) -> MacroCalls<T>
where
    T: ParseMacroCall,
{
    MacroCalls {
        calls: m
            .macro_calls
            .iter()
            .filter_map(|mc| {
                (&resolve_path(mc.path.to_vec(), cr, m, crates).is_ok_and(|p| &p == macro_path))
                    .then_some(())
                    .and_then(|_| T::parse(cr, m, crates, mc.args.clone()).ok())
            })
            .collect(),
        mods: m
            .mods
            .map_vec(|m| parse_macro_calls(macro_path, m, cr, crates)),
    }
}

pub fn update_macro_calls<T>(macro_calls: MacroCalls<T>, m: &mut AstMod) -> Vec<T>
where
    T: ParseMacroCall,
{
    let mut calls = macro_calls.calls;
    calls.iter().for_each(|c| c.update_mod(m));
    for (macro_calls, m) in macro_calls.mods.into_iter().zip(m.mods.iter_mut()) {
        calls.extend(update_macro_calls(macro_calls, m));
    }
    calls
}
