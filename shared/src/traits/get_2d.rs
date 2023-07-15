// Get for multidimensional vecetor
pub trait Get2D<T> {
    fn get<'a>(&'a self, i: usize, j: usize) -> Option<&'a T>;

    fn get_mut<'a>(&'a mut self, i: usize, j: usize) -> Option<&'a mut T>;
}

impl<T> Get2D<T> for Vec<Vec<T>> {
    fn get<'a>(&'a self, i: usize, j: usize) -> Option<&'a T> {
        <[Vec<T>]>::get(self, i).and_then(|v| v.get(j))
    }

    fn get_mut<'a>(&'a mut self, i: usize, j: usize) -> Option<&'a mut T> {
        <[Vec<T>]>::get_mut(self, i).and_then(|v| v.get_mut(j))
    }
}
