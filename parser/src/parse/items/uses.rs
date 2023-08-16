use diagnostic::{CatchErr, ResultsTrait};
use shared::{
    msg_result::ToMsgs,
    syn::{
        add_use_item,
        error::{SpannedResult, ToError},
    },
};

use crate::parse::{attributes::get_attributes_if_active, AstMod};

#[derive(Clone, Debug)]
pub struct AstUse {
    pub ident: String,
    pub first: String,
    pub path: Vec<String>,
    pub public: bool,
}

impl AstUse {
    pub fn from(mut path: Vec<String>, ident: &syn::Ident, vis: &syn::Visibility) -> Option<Self> {
        path.first().cloned().map(|first| {
            path.push(ident.to_string());
            Self {
                ident: ident.to_string(),
                first,
                path,
                public: match vis {
                    syn::Visibility::Public(_) => true,
                    syn::Visibility::Restricted(_) | syn::Visibility::Inherited => false,
                },
            }
        })
    }
}

// TODO: Ambiguous paths, :: paths
// TODO: type redeclarations

impl AstMod {
    pub fn visit_item_use(&mut self, i: syn::ItemUse) -> SpannedResult<()> {
        if let Some(attrs) = get_attributes_if_active(&i.attrs, &self.path, &Vec::new())? {
            let mut uses = Vec::new();
            self.visit_use_tree(i.tree, &mut Vec::new(), &mut uses)?;
            let public = match i.vis {
                syn::Visibility::Public(_) => true,
                syn::Visibility::Restricted(_) | syn::Visibility::Inherited => false,
            };
            uses.iter_mut().for_each(|u| u.public = public);
            self.uses.append(&mut uses);
        }

        Ok(())
    }

    fn visit_use_tree(
        &mut self,
        i: syn::UseTree,
        path: &mut Vec<String>,
        items: &mut Vec<AstUse>,
    ) -> SpannedResult<()> {
        Ok(match i {
            syn::UseTree::Path(i) => self.visit_use_path(i, path, items)?,
            syn::UseTree::Name(i) => self.visit_use_name(i, path, items)?,
            syn::UseTree::Rename(i) => self.visit_use_rename(i, path, items)?,
            syn::UseTree::Glob(i) => self.visit_use_glob(i, path, items)?,
            syn::UseTree::Group(i) => self.visit_use_group(i, path, items)?,
        })
    }

    fn visit_use_path(
        &mut self,
        i: syn::UsePath,
        path: &mut Vec<String>,
        items: &mut Vec<AstUse>,
    ) -> SpannedResult<()> {
        add_use_item(&self.path, path, i.ident.to_string());
        self.visit_use_tree(*i.tree, path, items)
    }

    fn visit_use_name(
        &mut self,
        i: syn::UseName,
        path: &mut Vec<String>,
        items: &mut Vec<AstUse>,
    ) -> SpannedResult<()> {
        add_use_item(&self.path, path, i.ident.to_string());
        items.push(AstUse {
            ident: path
                .last()
                .catch_err(i.error("Empty use path with 'self'"))?
                .to_string(),
            first: path
                .first()
                .catch_err(i.error("Empty use path with 'self'"))?
                .to_string(),
            path: path.to_vec(),
            public: false,
        });
        Ok(())
    }

    fn visit_use_rename(
        &mut self,
        i: syn::UseRename,
        path: &mut Vec<String>,
        items: &mut Vec<AstUse>,
    ) -> SpannedResult<()> {
        items.push(AstUse {
            ident: i.rename.to_string(),
            first: path
                .first()
                .catch_err(i.error("Empty use path with 'self'"))?
                .to_string(),
            path: [path.to_owned(), vec![i.ident.to_string()]].concat(),
            public: false,
        });
        Ok(())
    }

    fn visit_use_glob(
        &mut self,
        i: syn::UseGlob,
        path: &mut Vec<String>,
        items: &mut Vec<AstUse>,
    ) -> SpannedResult<()> {
        items.push(AstUse {
            ident: "*".to_string(),
            first: path
                .first()
                .catch_err(i.error("Empty use path with 'self'"))?
                .to_string(),
            path: path.to_owned(),
            public: false,
        });
        Ok(())
    }

    fn visit_use_group(
        &mut self,
        i: syn::UseGroup,
        path: &mut Vec<String>,
        items: &mut Vec<AstUse>,
    ) -> SpannedResult<()> {
        let mut errs = Vec::new();
        for i in i.items {
            self.visit_use_tree(i, &mut path.to_vec(), items)
                .record_errs(&mut errs);
        }
        errs.err_or(())
    }
}
