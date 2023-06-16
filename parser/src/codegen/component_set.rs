use shared::util::JoinMap;

use crate::{
    resolve::{
        ast_component_set::{self, LabelOp},
        ast_items::ItemsCrate,
        ast_resolve::Path,
    },
    validate::util::ItemIndex,
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
    pub fn validate_labels(root: &ast_component_set::LabelItem, crates: &Vec<ItemsCrate>) -> Self {
        let mut components = Vec::new();
        let expression = Self::get_labels(root, &mut components);
        let components = components.map_vec(|item_c| {
            (
                item_c.cr_idx,
                crates[item_c.cr_idx]
                    .components
                    .iter()
                    .position(|c| item_c == &c.path)
                    .unwrap(),
            )
        });
        Self {
            components,
            expression,
        }
    }

    fn get_labels(item: &ast_component_set::LabelItem, comps: &mut Vec<Path>) -> LabelItem {
        match item {
            ast_component_set::LabelItem::Item { not, ty } => LabelItem::Item {
                not: *not,
                component: {
                    comps.iter().position(|c| c == ty).unwrap_or_else(|| {
                        comps.push(ty.clone());
                        comps.len() - 1
                    })
                },
            },
            ast_component_set::LabelItem::Expression { op, items } => LabelItem::Expression {
                op: *op,
                items: items.map_vec(|item| Self::get_labels(item, comps)),
            },
        }
    }
}

#[derive(Debug)]
pub struct ComponentSet {
    pub path: Path,
    pub components: Vec<ItemIndex>,
    pub labels: Option<ComponentSetLabels>,
}

impl ComponentSet {
    pub fn parse(cs: &ast_component_set::ComponentSet, crates: &Vec<ItemsCrate>) -> Self {
        let labels = cs
            .labels
            .as_ref()
            .map(|root| ComponentSetLabels::validate_labels(root, crates));

        Self {
            path: cs.path.clone(),
            components: cs.args.map_vec(|item| {
                (
                    item.ty.cr_idx,
                    crates[item.ty.cr_idx]
                        .components
                        .iter()
                        .position(|c| item.ty == c.path)
                        .unwrap(),
                )
            }),
            labels,
        }
    }
}
