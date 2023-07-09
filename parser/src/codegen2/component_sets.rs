use proc_macro2::TokenStream;
use quote::quote;
use shared::util::JoinMap;

use crate::{
    match_ok,
    resolve::{
        util::{CombineMsgs, FlattenMsgs, MsgsResult, Zip2Msgs, Zip3Msgs},
        ComponentSet, EnginePaths,
    },
};

use super::Crates;

pub fn component_sets(
    cr_idx: usize,
    component_sets: &Vec<ComponentSet>,
    crates: &Crates,
) -> MsgsResult<Vec<TokenStream>> {
    let filter_fn = crates.get_syn_path(cr_idx, EnginePaths::Filter);
    let entity_set = crates.get_syn_path(cr_idx, EnginePaths::EntitySet);
    let entity = crates.get_syn_path(cr_idx, EnginePaths::Entity);

    match_ok!(Zip3Msgs, filter_fn, entity, entity_set, {
        component_sets.enumerate_map_vec(|(i, cs)| cs.quote(i, &filter_fn, &entity, &entity_set))
    })
}
