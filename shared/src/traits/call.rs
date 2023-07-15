// Trait for appling a function to a type as a member function
// Used for splitting tuples
pub trait Call<T, V> {
    fn call(&self, f: impl FnOnce(&T) -> V) -> V;

    fn call_into(self, f: impl FnOnce(T) -> V) -> V;
}

impl<T, V> Call<Self, V> for T {
    fn call(&self, f: impl FnOnce(&Self) -> V) -> V {
        f(&self)
    }

    fn call_into(self, f: impl FnOnce(Self) -> V) -> V {
        f(self)
    }
}
