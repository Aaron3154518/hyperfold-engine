use shared::util::JoinMap;

use crate::{
    resolve::util::ItemIndex,
    resolve::{
        component_set::{self, LabelOp},
        items_crate::ItemsCrate,
        path::ItemPath,
    },
};

#[derive(Debug)]
pub enum LabelItem {
    Item { not: bool, component: usize },
    Expression { op: LabelOp, items: Vec<LabelItem> },
}

#[derive(Debug)]
pub struct ComponentSetLabels {
    pub components: Vec<ItemIndex>,
    pub expression: LabelItem,
}

impl ComponentSetLabels {
    pub fn validate_labels(root: &component_set::AstLabelItem, crates: &Vec<ItemsCrate>) -> Self {
        let mut components = Vec::new();
        let expression = Self::get_labels(root, &mut components);
        let components = components.map_vec(|item_c| {
            (
                item_c.cr_idx,
                crates[item_c.cr_idx]
                    .find_component(item_c)
                    .map(|(i, _)| i)
                    .unwrap(),
            )
        });
        Self {
            components,
            expression,
        }
    }

    fn get_labels(item: &component_set::AstLabelItem, comps: &mut Vec<ItemPath>) -> LabelItem {
        match item {
            component_set::AstLabelItem::Item { not, ty } => LabelItem::Item {
                not: *not,
                component: {
                    comps.iter().position(|c| c == ty).unwrap_or_else(|| {
                        comps.push(ty.clone());
                        comps.len() - 1
                    })
                },
            },
            component_set::AstLabelItem::Expression { op, items } => LabelItem::Expression {
                op: *op,
                items: items.map_vec(|item| Self::get_labels(item, comps)),
            },
        }
    }
}

#[derive(Debug)]
pub struct ComponentSetItem {
    pub idx: ItemIndex,
    pub is_mut: bool,
}

#[derive(Debug)]
pub struct ComponentSet {
    pub path: ItemPath,
    pub components: Vec<ComponentSetItem>,
    pub labels: Option<ComponentSetLabels>,
}

impl ComponentSet {
    pub fn parse(cs: &component_set::AstComponentSet, crates: &Vec<ItemsCrate>) -> Self {
        let labels = cs
            .labels
            .as_ref()
            .map(|root| ComponentSetLabels::validate_labels(root, crates));

        Self {
            path: cs.path.clone(),
            components: cs.args.map_vec(|item| ComponentSetItem {
                idx: (
                    item.ty.cr_idx,
                    crates[item.ty.cr_idx]
                        .find_component(&item.ty)
                        .map(|(i, _)| i)
                        .unwrap(),
                ),
                is_mut: item.is_mut,
            }),
            labels,
        }
    }
}
