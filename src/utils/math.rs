use std::f32::consts::TAU;

#[macro_export]
macro_rules! f32 {
    ($e: expr) => {
        $e as f32
    };
}

pub trait NormalizeAngle {
    fn normalize_rad(self) -> Self;

    fn normalize_deg(self) -> Self;
}

impl NormalizeAngle for f32 {
    fn normalize_rad(self) -> Self {
        ((self % TAU) + TAU) % TAU
    }

    fn normalize_deg(self) -> Self {
        ((self % 360.0) + 360.0) % 360.0
    }
}
