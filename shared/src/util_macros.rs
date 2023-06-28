#[macro_export]
macro_rules! let_mut_vecs {
    ($id: ident) => {
        let mut $id = Vec::new();
    };

    ($($ids: ident),*) => {
        $(let mut $ids = Vec::new();)*
    };
}

#[macro_export]
macro_rules! hash_map {
    () => {
        std::collections::HashMap::new()
    };

    ({ $($k: expr => $v: expr),* }) => {
        [$(($k, $v)),*].into_iter().collect::<std::collections::HashMap<_, _>>()
    };
}
