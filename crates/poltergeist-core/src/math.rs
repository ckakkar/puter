use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn dot(self, b: Self) -> f32 {
        self.x * b.x + self.y * b.y
    }
    pub fn cross(self, b: Self) -> f32 {
        self.x * b.y - self.y * b.x
    }
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }
    pub fn normalized(self) -> Self {
        let l = self.length();
        if l > 1e-8 {
            self / l
        } else {
            Self::new(1.0, 0.0)
        }
    }
    pub fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }
    pub fn rotate(self, a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Self::new(c * self.x - s * self.y, s * self.x + c * self.y)
    }
    pub fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}
impl Add for Vec2 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y)
    }
}
impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y)
    }
}
impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, b: f32) -> Self {
        Self::new(self.x * b, self.y * b)
    }
}
impl Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, b: f32) -> Self {
        Self::new(self.x / b, self.y / b)
    }
}
impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        -1.0 * self
    }
}
impl Mul<Vec2> for f32 {
    type Output = Vec2;
    fn mul(self, b: Vec2) -> Vec2 {
        b * self
    }
}
impl AddAssign for Vec2 {
    fn add_assign(&mut self, b: Self) {
        *self = *self + b;
    }
}
impl SubAssign for Vec2 {
    fn sub_assign(&mut self, b: Self) {
        *self = *self - b;
    }
}
pub fn angular_velocity(w: f32, r: Vec2) -> Vec2 {
    r.perp() * w
}
