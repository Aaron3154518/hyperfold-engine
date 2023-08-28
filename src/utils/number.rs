use std::{
    cell::LazyCell,
    cmp::Ordering,
    f32::consts::PI,
    fmt::Display,
    ops::{Add, Div, Mul, Neg, Sub},
};

// Max number of e's to display for tetration
const MAX_E: i32 = 5;
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
fn sign(val: i32) -> i8 {
    (val > 0) as i8 - (val < 0) as i8
}

fn signf(val: f32) -> i8 {
    (val > 0.0) as i8 - (val < 0.0) as i8
}

fn pow10f(exp: f32) -> f32 {
    10.0_f32.powf(exp)
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

    // From scientific notation
    pub fn sci(base: f32, exp: i32) -> Self {
        Self::new(
            1,
            match base.abs() <= ERR {
                true => 0.0,
                false => base.abs().log10() + exp as f32,
            },
            signf(base),
        )
    }

    // From string
}

// 0
impl Default for Number {
    fn default() -> Self {
        Self {
            exp: 0.0,
            layer: 0,
            sign: 0,
        }
    }
}

// From float
impl From<f32> for Number {
    fn from(value: f32) -> Self {
        Self::new(0, value.abs(), signf(value))
    }
}

impl From<Number> for f32 {
    fn from(value: Number) -> Self {
        value.sign as f32 * {
            if &value >= &MAX_FLOAT {
                f32::MAX
            } else if &value <= &MIN_FLOAT {
                f32::MIN
            } else if value.layer == 0 {
                value.exp
            } else {
                let exp = (0..value.layer.abs()).fold(value.exp, |e, _| pow10f(e));
                pow10f(match value.layer > 0 {
                    true => exp,
                    false => -exp,
                })
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
                    exp = pow10f(exp);
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
                    exp = pow10f(exp);
                    layer = 0;
                }
                _ => {
                    exp = pow10f(exp);
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
impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        (self.exp - other.exp).abs() < ERR && self.layer == other.layer && self.sign == other.sign
    }
}

impl Eq for Number {}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            return Ordering::Equal;
        }
        match if self.sign * other.sign != 1 {
            self.sign < other.sign
        } else if self.layer != other.layer {
            self.layer < self.layer
            // At this point the layers and signs are equal
        } else {
            (self.exp < other.exp) == (self.sign == 1)
        } {
            true => Ordering::Less,
            false => Ordering::Greater,
        }
    }
}

impl Neg for Number {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        self.sign *= -1;
        self
    }
}

impl Add for Number {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        if rhs.sign == 0 {
            self
        } else if self.sign == 0 {
            rhs
        // If either number has a layer above 1, addition is useless
        } else if self.layer.abs() > 1 || rhs.layer.abs() > 1 {
            match self.abs() > rhs.abs() {
                true => self,
                false => rhs,
            }
        } else {
            let [mut x, mut y] = [&self, &rhs].map(|n| match n.layer == 0 {
                true => n.exp.log10(),
                false => n.layer as f32 * n.exp,
            });
            // Same sign
            if self.sign * rhs.sign == 1 {
                // 10^x + 10^y = 10^(log(10^(y-x) + 1) + x)
                // -10^x + -10^y = -(10^x + 10^y)
                self.exp = (pow10f(y - x) + 1.0).log10() + x;
            // Opposite sign
            } else {
                // 10^x + -10^y = 10^(log(1 - 10^(y-x)) + x)
                // -10^x + 10^y = -(10^x + -10^y)
                match x == y {
                    true => self.sign = 0,
                    false => {
                        // Let's make y < x true. if y > x, swap x and y and negate
                        if y > x {
                            [x, y] = [y, x];
                            self.sign *= -1;
                        }
                        self.exp = (1.0 - pow10f(y - x)).log10() + x;
                    }
                }
            }
            self.layer = 1;
            self.balance()
        }
    }
}

impl Sub for Number {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + -rhs
    }
}

impl Mul for Number {
    type Output = Self;

    fn mul(mut self, rhs: Self) -> Self::Output {
        if self.sign == 0 || rhs.sign == 0 {
            self.sign = 0;
            self.balance()
        } else {
            let mut n = (self.exponent() + rhs.exponent()).pow10();
            n.sign = self.sign * rhs.sign;
            n
        }
    }
}

impl Div for Number {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.reciprocal()
    }
}

// Helper operations
impl Number {
    // |n|
    pub fn abs(mut self) -> Self {
        self.sign = self.sign.abs();
        self
    }

    // log10|n|
    pub fn exponent(self) -> Self {
        if self.sign == 0 {
            self
        } else if self.layer == 0 {
            Self::new(0, self.exp.log10(), 1)
        } else {
            Self::new(self.layer.abs() - 1, self.exp, sign(self.layer))
        }
        .balance()
    }

    // 1/n
    pub fn reciprocal(mut self) -> Self {
        if self.sign == 0 {
            panic!("Divide by zero error for Number");
        }

        match self.layer == 0 {
            true => self.exp = 1.0 / self.exp,
            false => self.layer *= -1,
        }
        self.balance()
    }

    // n^m
    pub fn pow(self, exp: Self) -> Self {
        if self.sign == 0 {
            match exp.sign {
                1 => self,
                0 => panic!("0^0: Divide by zero error for Number"),
                _ => panic!("0^{exp}: Divide by zero error for Number"),
            }
        } else if exp.sign == 0 {
            Self::new(0, 1.0, 1)
        } else {
            let n = (self.exponent() * exp).pow10();
            // If our base is negative, we only want the real component of the answer
            // If the power's layer is greater than 0, Math::rounding will cause pow to
            //     be a multiple of 10 and thus pow*pi will be a multiple of 2pi
            // If the layer is less than 0, pow * pi will essentially be 0*2pi
            match self.sign == -1 && exp.layer == 0 {
                true => n * (exp.exp * PI).cos().into(),
                false => n,
            }
        }
    }

    // 10^n
    pub fn pow10(self) -> Self {
        match self.sign == 0 || self.layer == 0 {
            true => Self::new(0, 1.0, 1),
            false => Self::new(self.sign as i32 * (self.layer + 1), self.exp, 1),
        }
        .balance()
    }

    // log10(n)
    pub fn log10(self) -> Self {
        match self.sign {
            1 => self.exponent(),
            0 => panic!("log(0): Cannot take log of 0 for Number"),
            _ => panic!("log({self}): Cannot take log of negative number for Number"),
        }
    }

    // logb(n)
    pub fn log(self, base: Self) -> Self {
        (self.log10() / base.log10()).balance()
    }

    // sqrt(n)
    pub fn sqrt(self) -> Self {
        self.log(0.5.into())
    }

    // Floor/Ceiling
    pub fn floor(mut self) -> Self {
        match self.layer {
            0 => self.exp = self.exp.floor(),
            l if l < 0 => self = if self.sign == -1 { -1.0 } else { 0.0 }.into(),
            _ => (),
        }
        self.balance()
    }

    pub fn ceil(mut self) -> Self {
        match self.layer {
            0 => self.exp = self.exp.ceil(),
            l if l < 0 => self = if self.sign == 1 { 1.0 } else { 0.0 }.into(),
            _ => (),
        }
        self.balance()
    }
}

// Print
impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.sign == -1 {
            f.write_str("-")?;
        }
        if self.layer < 0 {
            f.write_str("1/")?;
        }
        match self.layer.abs() {
            1 => write!(f, "{:.2}e{}", pow10f(self.exp % 1.0), self.exp as i32),
            _ => {
                match self.layer.abs() < MAX_E {
                    true => f.write_str(&"e".repeat(self.layer.unsigned_abs() as usize)),
                    false => write!(f, "10^^{}^", self.layer),
                }?;
                match self.exp == 0. || (0.01 <= self.exp && self.exp < 1000.0) {
                    true => write!(f, "{:.2}", self.exp),
                    false => {
                        let val = self.exp.log10();
                        write!(f, "{:.2}e{}", pow10f(val % 1.0), val as i32)
                    }
                }
            }
        }
    }
}
