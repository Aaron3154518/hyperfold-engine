use proc_macro2::TokenStream;
use quote::quote;
use shared::util::{JoinMap, JoinMapInto};

use crate::{
    match_ok,
    resolve::{
        constants::{component_set_var, global_var},
        util::{CombineMsgs, MsgsResult, ToMsgsResult, Zip2Msgs},
        FnArgType, FnArgs, ItemSystem, Items,
    },
};

use super::{
    idents::CodegenIdents,
    util::{vec_to_path, Quote},
    Crates,
};

pub fn systems(cr_idx: usize, items: &Items, crates: &Crates) -> MsgsResult<Vec<TokenStream>> {
    let globals_var = CodegenIdents::GenGFoo.to_ident();
    let comps_var = CodegenIdents::GenCFoo.to_ident();
    let events_var = CodegenIdents::GenEFoo.to_ident();
    let globals_type = CodegenIdents::GFoo.to_ident();
    let comps_type = CodegenIdents::CFoo.to_ident();
    let events_type = CodegenIdents::EFoo.to_ident();
    let event_arg = CodegenIdents::GenE.to_ident();

    items.systems.map_vec(|system| {
        let func_name = crates
            .get_item_path(cr_idx, &system.path)
            .map(|v| vec_to_path(v))
            .to_msg_vec();
        system
            .validate(items)
            .zip(func_name)
            .map(|(args, func_name)| match args {
                FnArgs::Init { mut globals } => {
                    globals.sort_by_key(|g| g.arg_idx);
                    let func_args = globals.map_vec(|g| {
                        let mut_tok = if g.is_mut { quote!(mut) } else { quote!() };
                        let var = global_var(g.idx);
                        quote!(&#mut_tok self.#globals_var.#var)
                    });
                    quote!(#func_name(#(#func_args),*))
                }
                FnArgs::System {
                    event,
                    globals,
                    component_sets,
                } => {
                    let num_args = 1 + globals.len() + component_sets.len();
                    let mut func_args = (0..num_args).map_vec_into(|_| quote!());
                    func_args[event.arg_idx] = event_arg.quote();
                    for g in globals {
                        func_args[g.arg_idx] = {
                            let mut_tok = if g.is_mut { quote!(mut) } else { quote!() };
                            let var = global_var(g.idx);
                            quote!(&#mut_tok #globals_var.#var)
                        };
                    }
                    for cs in component_sets {
                        func_args[cs.arg_idx] = component_set_var(cs.idx).quote()
                    }
                    quote!(
                        |#comps_var: &mut #comps_type, #globals_var: &mut #globals_type, #events_var: &mut #events_type| {
                            #func_name(#(#func_args),*)
                        }
                    )
                }
            })
    })
    .filter_vec_into(|r| r.is_ok())
    .combine_msgs()
}
