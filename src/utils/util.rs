use std::{
    array,
    cmp::Ordering,
    f32::consts::{PI, TAU},
    sync::LazyLock,
};

use uuid::Uuid;

pub const HALF_PI: f32 = PI / 2.0;
pub const DEG_TO_RAD: f32 = PI / 180.0;
pub const F32_ERR: f32 = 1e-10;

pub fn normalize_rad(rad: f32) -> f32 {
    ((rad % TAU) + TAU) % TAU
}

// Math on ints that returns floats
pub trait IntMath {
    fn sqrt(self) -> f32;
}

impl IntMath for u32 {
    fn sqrt(self) -> f32 {
        (self as f32).sqrt()
    }
}

impl IntMath for i32 {
    fn sqrt(self) -> f32 {
        (self as f32).sqrt()
    }
}

// Math on floats that returns ints
pub trait FloatMath {
    fn round_i32(self) -> i32;

    fn round_to_i32(self, v: f32) -> i32;

    fn floor_i32(self) -> i32;

    fn ceil_i32(self) -> i32;
}

impl FloatMath for f32 {
    fn round_i32(self) -> i32 {
        self.round() as i32
    }

    fn round_to_i32(self, v: f32) -> i32 {
        ((self / v).round() * v) as i32
    }

    fn floor_i32(self) -> i32 {
        self.floor() as i32
    }

    fn ceil_i32(self) -> i32 {
        self.ceil() as i32
    }
}

// Cross product
pub trait CrossProduct<'a, T, R> {
    fn cross(&'a self, t: &'a T) -> R;
}

struct AssertProduct<const N: usize, const M: usize, const R: usize>;
impl<const N: usize, const M: usize, const R: usize> AssertProduct<N, M, R> {
    const OK: () = assert!(
        R == M * N,
        "In cross(): Output array size does match product of input array sizes",
    );
}

impl<'a, T, U, const N: usize, const M: usize, const R: usize>
    CrossProduct<'a, [U; M], [(&'a T, &'a U); R]> for [T; N]
{
    fn cross(&'a self, us: &'a [U; M]) -> [(&'a T, &'a U); R] {
        let () = AssertProduct::<N, M, R>::OK;
        array::from_fn(|i| (&self[i / N], &us[i % M]))
    }
}

// Traits for implementing downcasting
pub trait AsAny {
    fn as_any<'a>(&'a self) -> &'a dyn std::any::Any;

    fn as_any_mut<'a>(&'a mut self) -> &'a mut dyn std::any::Any;
}

pub trait AsType<T: 'static>: AsAny {
    fn as_type<'a>(&'a self) -> Option<&'a T> {
        self.as_any().downcast_ref()
    }

    fn as_type_mut<'a>(&'a mut self) -> Option<&'a mut T> {
        self.as_any_mut().downcast_mut()
    }

    fn try_as<'a>(&'a self, f: impl FnOnce(&'a T)) -> bool {
        self.as_type().map(|t| f(t)).is_some()
    }

    fn try_as_mut<'a>(&'a mut self, f: impl FnOnce(&'a mut T)) -> bool {
        self.as_type_mut().map(|t| f(t)).is_some()
    }
}

pub trait TryAsType<U, T>
where
    T: 'static,
    U: AsType<T> + ?Sized,
{
    fn try_as<'a>(self, u: &'a U, f: impl FnOnce(&'a T)) -> bool;

    fn try_as_mut<'a>(self, u: &'a mut U, f: impl FnOnce(&'a mut T)) -> bool;
}

impl<U, T> TryAsType<U, T> for bool
where
    T: 'static,
    U: AsType<T> + ?Sized,
{
    fn try_as<'a>(self, u: &'a U, f: impl FnOnce(&'a T)) -> bool {
        self || u.try_as(f)
    }

    fn try_as_mut<'a>(self, u: &'a mut U, f: impl FnOnce(&'a mut T)) -> bool {
        self || u.try_as_mut(f)
    }
}

#[macro_export]
macro_rules! impl_as_any_for_trait {
    ($tr: ident) => {
        impl<T> crate::utils::util::AsAny for T
        where
            T: $tr + 'static,
        {
            fn as_any<'a>(&'a self) -> &'a dyn std::any::Any {
                self
            }

            fn as_any_mut<'a>(&'a mut self) -> &'a mut dyn std::any::Any {
                self
            }
        }

        impl<T> crate::utils::util::AsType<T> for dyn $tr where T: $tr + 'static {}
    };
}

#[macro_export]
macro_rules! impl_as_any_for_type {
    ($ty: ident) => {
        impl crate::utils::util::AsAny for $ty {
            fn as_any<'a>(&'a self) -> &'a dyn std::any::Any {
                self
            }

            fn as_any_mut<'a>(&'a mut self) -> &'a mut dyn std::any::Any {
                self
            }
        }
    };
}

// Function to create static uuids
pub trait UuidTrait {
    fn new() -> Uuid;

    // TODO: const
    fn create() -> LazyLock<Uuid>;
}

impl UuidTrait for Uuid {
    fn new() -> Uuid {
        Uuid::new_v4()
    }

    fn create() -> LazyLock<Uuid> {
        LazyLock::new(|| Uuid::new_v4())
    }
}

// Function to order multiple comparisons
pub fn cmp<const N: usize>(cmps: [Ordering; N]) -> Ordering {
    for cmp in cmps {
        match cmp {
            Ordering::Less => return Ordering::Less,
            Ordering::Greater => return Ordering::Greater,
            Ordering::Equal => (),
        }
    }
    Ordering::Equal
}
