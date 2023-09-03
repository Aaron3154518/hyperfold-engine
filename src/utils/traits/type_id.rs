use std::any::TypeId;

pub trait Id {
    fn type_id(&self) -> TypeId;
}

impl<T: 'static> Id for T {
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}
