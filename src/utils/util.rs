use std::{
    array,
    f32::consts::{PI, TAU},
};

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
