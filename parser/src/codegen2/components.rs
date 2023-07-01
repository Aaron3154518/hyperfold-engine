use proc_macro2::TokenStream;
use quote::quote;
use shared::util::{JoinMap, JoinMapInto};

use crate::{
    codegen2::idents::CodegenIdents,
    resolve::{
        constants::component_var,
        util::{CombineMsgs, MsgsResult, ToMsgsResult, Zip5Msgs},
        EngineGlobals, EnginePaths, GetPaths, ItemComponent,
    },
    zip_msgs,
};

use super::{util::vec_to_path, Codegen, Crates};

impl Codegen {
    pub fn components(
        cr_idx: usize,
        components: &Vec<ItemComponent>,
        crates: &Crates,
    ) -> MsgsResult<TokenStream> {
        let struct_name = CodegenIdents::CFoo.to_ident();
        let vars = (0..components.len()).map_vec(|i| component_var(i));
        let types = components
            .map_vec(|c| crates.get_path(cr_idx, &c.path).map(|v| vec_to_path(v)))
            .combine_msgs();

        let entity_set = crates
            .path_from(cr_idx, EnginePaths::EntitySet)
            .map(|v| vec_to_path(v))
            .to_msg_vec();
        let entity_trash = crates
            .path_from(cr_idx, EngineGlobals::EntityTrash)
            .map(|v| vec_to_path(v))
            .to_msg_vec();
        let entity_map = crates
            .path_from(cr_idx, EnginePaths::EntityMap)
            .map(|v| vec_to_path(v))
            .to_msg_vec();
        let singleton = crates
            .path_from(cr_idx, EnginePaths::Singleton)
            .map(|v| vec_to_path(v))
            .to_msg_vec();

        let i = zip_msgs!(types, entity_set, entity_trash, entity_map, singleton, {
            0
        });

        types
            .zip(entity_set, entity_trash, entity_map, singleton)
            .map(|(types, entity_set, entity_trash, entity_map, singleton)| {});

        let news = entity_map.map(|entity_map| {
            let news = components.map_vec(|c| {
                if c.args.is_singleton {
                    quote!(None)
                } else {
                    quote!(#entity_map::new())
                }
            });
            let adds = components.map_vec(|c| {
                if c.args.is_singleton {
                    quote!(self.#var = Some(#singleton::new(e, t)))
                } else {
                    quote!(self.#var.insert(e, t);)
                }
            });
            let appends = components.map_vec(|c| {
                if c.args.is_singleton {
                    quote!(None)
                } else {
                    quote!(#entity_map::new())
                }
            });
            let removes = components.map_vec(|c| {
                if c.args.is_singleton {
                    quote!(None)
                } else {
                    quote!(#entity_map::new())
                }
            });
            news
        });

        types
            .zip(entity_set)
            .zip(entity_trash)
            .map(|((types, entity_set), entity_trash)| {
                quote!(
                    pub struct #struct_name {
                        eids: #entity_set,
                        #(#vars: #types),*
                    }

                    impl #struct_name {
                        fn new() -> Self {
                            Self {
                                eids: #entity_set::new(),
                                #(#vars: #cfoo_news),*
                            }
                        }

                        fn append(&mut self, cm: &mut Self) {
                            self.eids.extend(cm.eids.drain());
                            #(#cfoo_appends)*
                        }

                        fn remove(&mut self, tr: &mut #entity_trash) {
                            for eid in tr.0.drain(..) {
                                self.eids.remove(&eid);
                                #(#cfoo_removes)*
                            }
                        }
                    }
                )
            })
    }
}
