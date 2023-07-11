use proc_macro2::TokenStream;
use quote::quote;
use shared::util::JoinMap;

use crate::{
    match_ok,
    resolve::{
        util::{CombineMsgs, FlattenMsgs, MsgsResult, Zip2Msgs, Zip3Msgs},
        ComponentSet, ENGINE_PATHS,
    },
};

use super::Crates;

pub fn component_sets(
    cr_idx: usize,
    component_sets: &Vec<ComponentSet>,
    crates: &Crates,
) -> MsgsResult<Vec<TokenStream>> {
    let filter_fn = crates.get_syn_path(cr_idx, &ENGINE_PATHS.filter);
    let entity_set = crates.get_syn_path(cr_idx, &ENGINE_PATHS.entity_set);
    let entity = crates.get_syn_path(cr_idx, &ENGINE_PATHS.entity);

    match_ok!(Zip3Msgs, filter_fn, entity, entity_set, {
        component_sets.enumerate_map_vec(|(i, cs)| cs.quote(i, &filter_fn, &entity, &entity_set))
    })
}
