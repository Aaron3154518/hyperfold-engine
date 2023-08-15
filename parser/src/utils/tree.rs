pub struct Tree<T> {
    pub root: T,
    pub children: Vec<Tree<T>>,
}

pub type FlatTree<T> = Vec<(T, Vec<usize>)>;

pub trait FlattenTree<T> {
    fn flatten(self) -> FlatTree<T>;
}

impl<T> Tree<T> {
    pub fn new(root: T) -> Self {
        Self {
            root,
            children: Vec::new(),
        }
    }

    // Flattens bottom up (lrn)
    fn flatten_impl(self, mut arr: FlatTree<T>) -> FlatTree<T> {
        let mut idxs = Vec::new();
        for child in self.children {
            arr = child.flatten_impl(arr);
            idxs.push(arr.len() - 1);
        }
        arr.push((self.root, idxs));
        arr
    }
}

impl<T> FlattenTree<T> for Tree<T> {
    fn flatten(self) -> FlatTree<T> {
        self.flatten_impl(Vec::new())
    }
}

impl<T> FlattenTree<T> for Vec<Tree<T>> {
    fn flatten(self) -> FlatTree<T> {
        self.into_iter()
            .fold(Vec::new(), |arr, child| child.flatten_impl(arr))
    }
}
