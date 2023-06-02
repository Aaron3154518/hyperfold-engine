#[macro_export]
macro_rules! let_mut_vecs {
    ($id: ident) => {
        let mut $id = Vec::new();
    };

    ($($ids: ident),*) => {
        $(let mut $ids = Vec::new();)*
    };
}
