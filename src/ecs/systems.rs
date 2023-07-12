#[macro_export]
macro_rules! components {
        // Op => e
        (@op) => {};

        // Op => && NoOp
        (@op && $($tail: tt)*) => {
            $crate::components!(@no_op $($tail)*);
        };

        // Op => || NoOp
        (@op || $($tail: tt)*) => {
            $crate::components!(@no_op $($tail)*);
        };

        // NoOp => (NoOp) Op
        (@no_op ($($inner: tt)*) $($tail: tt)*) => {
            $crate::components!(@no_op $($inner)*);
            $crate::components!(@op $($tail)*);
        };

        // Ty => :: ident Ty
        (@ty ($($ty: ident),+) :: $i: ident $($tail: tt)*) => {
            $crate::components!(@ty ($($ty),*,$i) $($tail)*);
        };

        // Ty => Op
        (@ty ($($ty: ident),+) $($tail: tt)*) => {
            const _: std::marker::PhantomData<$($ty)::*> = std::marker::PhantomData;
            $crate::components!(@op $($tail)*);
        };

        // NoOp => ident Ty Op
        (@no_op $i: ident $($tail: tt)*) => {
            $crate::components!(@ty ($i) $($tail)*);
        };

        // NoOp => ! NoOp
        (@no_op ! $($tail: tt)*) => {
            $crate::components!(@no_op $($tail)*);
        };

        // S => e
        (@labels) => {};

        // S => NoOp
        (@labels $($tts: tt)+) => {
            $crate::components!(@no_op $($tts)*);
        };

        (labels ($($labels: tt)*), $name: ident $(,$n: ident: $t: ty)*) => {
            $crate::components!(@labels $($labels)*);
            $crate::components!($name $(,$n: $t)*);
        };

        ($name: ident $(,$n: ident: $t: ty)*) => {
            pub struct $name<'a> {
                pub eid: &'a crate::_engine::Entity
                $(,pub $n: $t)*
            }

            impl<'a> $name<'a> {
                pub fn new(eid: &'a crate::_engine::Entity $(,$n: $t)*) -> Self {
                    Self { eid $(,$n)* }
                }
            }
        };
    }

pub type Entities<T> = Vec<T>;
