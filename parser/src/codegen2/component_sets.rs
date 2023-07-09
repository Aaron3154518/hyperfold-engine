use proc_macro2::TokenStream;
use quote::quote;
use shared::util::JoinMap;

use crate::{
    match_ok,
    resolve::{
        constants::component_set_fn,
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
    let intersect_fn = crates.get_syn_path(cr_idx, EnginePaths::Intersect);

    match_ok!(Zip2Msgs, filter_fn, intersect_fn, {
        component_sets
            .enumerate_map_vec(|(i, cs)| {
                crates
                    .get_item_syn_path(cr_idx, &cs.path)
                    .map(|ty| cs.quote(i, &ty, &filter_fn, &intersect_fn))
            })
            .combine_msgs()
    })
    .flatten_msgs()
}
