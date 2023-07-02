use proc_macro2::TokenStream;
use quote::quote;
use shared::util::JoinMap;

use crate::resolve::{
    constants::{component_set_var, global_var},
    util::{CombineMsgs, MsgsResult},
    FnArgType, ItemSystem, Items,
};

use super::{
    idents::CodegenIdents,
    util::{vec_to_path, Quote},
    Crates,
};

pub fn systems(cr_idx: usize, items: &Items, crates: &Crates) -> MsgsResult<Vec<TokenStream>> {
    let globals = CodegenIdents::GenGFoo.to_ident();
    let components = CodegenIdents::GenCFoo.to_ident();
    let events = CodegenIdents::GenEFoo.to_ident();
    let globals_type = CodegenIdents::GFoo.to_ident();
    let comps_type = CodegenIdents::CFoo.to_ident();
    let events_type = CodegenIdents::EFoo.to_ident();
    let event_arg = CodegenIdents::GenE.to_ident();

    let codegen_system = |system: &ItemSystem| {
        let func_name = crates
            .get_item_path(cr_idx, &system.path)
            .map(|v| vec_to_path(v));
        let func_args = system.args.map_vec(|arg| match arg.ty {
            FnArgType::Event(i) => event_arg.quote(),
            FnArgType::Global(i) => {
                let mut_tok = if arg.is_mut { quote!(mut) } else { quote!() };
                let var = global_var(i);
                quote!(&#mut_tok #globals.#var)
            }
            FnArgType::Entities(i) => component_set_var(i).quote(),
            FnArgType::VecEntities(i) => component_set_var(i).quote(),
        });
        func_name.map(|func_name| quote!(
                |#components: &mut #comps_type, #globals: &mut #globals_type, #events: &mut #events_type| {
                    #func_name(#(#func_args),*)
                }
            ))
    };

    items.systems.map_vec(codegen_system).combine_msgs()
}
