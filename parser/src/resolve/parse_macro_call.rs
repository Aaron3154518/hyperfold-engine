use proc_macro2::TokenStream;
use shared::util::JoinMap;

use crate::parse::{AstCrate, AstMod};

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

impl<T> std::fmt::Debug for MacroCalls<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacroCalls")
            .field("calls", &self.calls)
            .field("mods", &self.mods)
            .finish()
    }
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
    // eprintln!("{:#?}", m.macro_calls.map_vec(|mc| mc.path.to_vec()));
    // eprintln!(
    //     "{:#?}",
    //     m.macro_calls
    //         .map_vec(|mc| resolve_path(mc.path.to_vec(), cr, m, crates))
    // );
    // MacroCalls {
    //     calls: m
    //         .macro_calls
    //         .iter()
    //         .filter_map(|mc| {
    //             (&resolve_path(mc.path.to_vec(), cr, m, crates).is_ok_and(|p| &p == macro_path))
    //                 .then_some(())
    //                 .and_then(|_| T::parse(cr, m, crates, mc.args.clone()).ok())
    //         })
    //         .collect(),
    //     mods: m
    //         .mods
    //         .map_vec(|m| parse_macro_calls(macro_path, m, cr, crates)),
    // }
    MacroCalls {
        calls: Vec::new(),
        mods: Vec::new(),
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
