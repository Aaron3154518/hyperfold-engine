use std::cell::LazyCell;

// Max number of e's to display for tetration
const MAX_E: i8 = 5;
// Error tolerance when comparing floats
const ERR: f32 = 1e-10;
// Value at which to increase the layer
const CONV_UP: f32 = 1e10;
// Value at which to decrease the layer
const CONV_DOWN: f32 = 10.0;
// Value at which to make the layer negative
const CONV_NEG: f32 = 1e-10;

const MAX_FLOAT: LazyCell<Number> = LazyCell::new(|| f32::MAX.into());
const MIN_FLOAT: LazyCell<Number> = LazyCell::new(|| f32::MIN.into());

// -1, 0, 1
fn sign(val: f32) -> i8 {
    (val > 0.0) as i8 - (val < 0.0) as i8
}

#[derive(Debug, Copy, Clone)]
pub struct Number {
    exp: f32,
    layer: i32,
    // -1, 0, 1
    sign: i8,
}

// Constructors
impl Number {
    pub fn new(layer: i32, exp: f32, sign: i8) -> Self {
        Self { exp, layer, sign }.balance()
    }

    pub fn pow(base: f32, exp: i32) -> Self {
        Self::new(
            1,
            match base.abs() <= ERR {
                true => 0.0,
                false => base.abs().log10() + exp as f32,
            },
            sign(base),
        )
    }
}

impl Default for Number {
    fn default() -> Self {
        Self {
            exp: 0.0,
            layer: 0,
            sign: 0,
        }
    }
}

impl From<f32> for Number {
    fn from(value: f32) -> Self {
        Self::new(0, value.abs(), sign(value))
    }
}

impl From<Number> for f32 {
    fn from(value: Number) -> Self {
        value.sign * {
            if value >= MAX_FLOAT {
                f32::MAX
            } else if value <= MIN_FLOAT {
                f32::MIN
            } else if value.layer == 0 {
                value.exp
            } else {
                let exp = (0..value.layer.abs()).fold(value.exp, |e, _| 10 ^ e);
                10 ^ match value.layer > 0 {
                    true => exp,
                    false => -exp,
                }
            }
        }
    }
}

// Maintaining state
impl Number {
    // 10 <= exp < 1e10
    pub fn balance(self) -> Self {
        let Self {
            mut exp,
            mut layer,
            mut sign,
        } = self;
        // 0 is already balanced
        if sign == 0 || (layer == 0 && exp.abs() <= ERR) {
            return Default::default();
        }
        // Need to make exp positive
        if exp < 0.0 {
            match layer {
                // exp is the value, move negative into sign
                0 => {
                    exp *= -1.0;
                    sign *= -1;
                }
                // 10^exp, move negative into layer
                1 => {
                    exp *= -1.0;
                    layer = -1;
                }
                -1 => {
                    exp *= -1.0;
                    layer = 1;
                }
                // 0 < exp = 10^exp < 1
                _ => {
                    exp = 10.0_f32.powf(exp);
                    // Move the layer towards 0
                    layer += match layer > 0 {
                        true => -1,
                        false => 1,
                    };
                }
            }
        }
        // exp is small, move its value into layer
        if exp < CONV_DOWN {
            match layer {
                0 => {
                    if exp <= CONV_NEG {
                        exp = -exp.log10();
                        layer = -1;
                    }
                }
                -1 => {
                    exp = 10.0_f32.powf(exp);
                    layer = 0;
                }
                _ => {
                    exp = 10.0_f32.powf(exp);
                    layer += match layer > 0 {
                        true => -1,
                        false => 1,
                    };
                }
            }
        }
        // exp is big, move its value into layer
        if exp >= CONV_UP {
            exp = exp.log10();
            layer += match layer >= 0 {
                true => 1,
                false => -1,
            };
        }

        Self::new(layer, exp, sign)
    }
}

// Operators
impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.exp.partial_cmp(&other.exp) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.layer.partial_cmp(&other.layer) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.sign.partial_cmp(&other.sign)
    }
}
