// Add element to vec in place
pub trait PushInto<T> {
    fn push_into(self, t: T) -> Self;

    fn push_item(&mut self, t: T) -> &mut Self;
}

impl<T> PushInto<T> for Vec<T> {
    fn push_into(mut self, t: T) -> Self {
        self.push(t);
        self
    }

    fn push_item(&mut self, t: T) -> &mut Self {
        self.push(t);
        self
    }
}
