//! # Mathematics

use std::ops::{Add, Mul, Sub};



#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct Aabb2D<V> {
    pub x_min: V,
    pub x_max: V,
    pub y_min: V,
    pub y_max: V,
}

impl Aabb2D<f32> {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    #[inline]
    pub const fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            x_min: min_x,
            x_max: max_x,
            y_min: min_y,
            y_max: max_y,
        }
    }

    #[inline]
    pub const fn from_min_max(min: Xy<f32>, max: Xy<f32>) -> Self {
        Self {
            x_min: min.x,
            x_max: max.x,
            y_min: min.y,
            y_max: max.y,
        }
    }

    #[inline]
    pub const fn from_size(size: Xy<f32>) -> Self {
        Self {
            x_min: 0.0,
            x_max: size.x,
            y_min: 0.0,
            y_max: size.y,
        }
    }

    #[inline]
    pub const fn size(&self) -> Xy<f32> {
        Xy::new(self.x_max - self.x_min, self.y_max - self.y_min)
    }

    #[inline]
    pub const fn position(&self) -> Xy<f32> {
        Xy::new(self.x_min, self.y_min)
    }

    #[inline]
    pub const fn abs(&self) -> Self {
        let Self {
            x_min,
            y_min,
            x_max,
            y_max,
        } = *self;
        Self::new(
            x_min.min(x_max),
            y_min.min(y_max),
            x_min.max(x_max),
            y_min.max(y_max),
        )
    }

    #[inline]
    pub fn set_size(&mut self, size: Xy<f32>) {
        self.x_max = self.x_min + size.x;
        self.y_max = self.y_min + size.y;
    }

    pub const fn translate(&self, amount: Xy<f32>) -> Self {
        Self {
            x_min: self.x_min + amount.x,
            x_max: self.x_max + amount.x,
            y_min: self.y_min + amount.y,
            y_max: self.y_max + amount.y,
        }
    }

    #[inline]
    pub const fn intersect(&self, other: Self) -> Self {
        let x_min = self.x_min.max(other.x_min);
        let y_min = self.y_min.max(other.y_min);
        let x_max = self.x_max.min(other.x_max);
        let y_max = self.y_max.min(other.y_max);
        Self::new(x_min, y_min, x_max.max(x_min), y_max.max(y_min))
    }

    #[inline]
    pub const fn union(&self, other: Self) -> Self {
        Self::new(
            self.x_min.min(other.x_min),
            self.y_min.min(other.y_min),
            self.x_max.max(other.x_max),
            self.y_max.max(other.y_max),
        )
    }

    #[inline]
    pub const fn contains(&self, point: Xy<f32>) -> bool {
        point.x >= self.x_min
            && point.x <= self.x_max
            && point.y >= self.y_min
            && point.y <= self.y_max
    }

    #[inline]
    pub const fn overlaps(&self, other: Self) -> bool {
        self.x_min <= other.x_max
            && self.x_max >= other.x_min
            && self.y_min <= other.y_max
            && self.y_max >= other.y_min
    }

    #[inline]
    pub const fn add_insets(self, other: Self) -> Self {
        let other = other.abs();
        Self::new(
            other.x_min - self.x_min,
            other.y_min - self.y_min,
            other.x_max + self.x_max,
            other.y_max + self.y_max,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(C)]
pub struct Xy<V> {
    pub x: V,
    pub y: V,
}

impl<V> Xy<V> {
    #[inline]
    pub const fn new(x: V, y: V) -> Self {
        Self { x, y }
    }
}

impl<V: Copy> Xy<V> {
    pub const fn value_for_axis(&self, axis: Axis) -> V {
        match axis {
            Axis::Horizontal => self.x,
            Axis::Vertical => self.y,
        }
    }
}

impl Xy<f32> {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    #[inline]
    pub const fn round(self) -> Self {
        Self::new(self.x.round(), self.y.round())
    }
}

impl<V: Add<V, Output = V>> Add for Xy<V> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<V: Sub<V, Output = V>> Sub for Xy<V> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D([f32; 6]);

impl Transform2D {
    /// The identity transform.
    pub const IDENTITY: Transform2D = Transform2D::scale(1.0);

    /// A transform that is flipped on the y-axis. Useful for converting between
    /// y-up and y-down spaces.
    pub const FLIP_Y: Transform2D = Transform2D::new([1.0, 0., 0., -1.0, 0., 0.]);

    /// A transform that is flipped on the x-axis.
    pub const FLIP_X: Transform2D = Transform2D::new([-1.0, 0., 0., 1.0, 0., 0.]);

    /// Construct an affine transform from coefficients.
    #[inline(always)]
    pub const fn new(c: [f32; 6]) -> Transform2D {
        Transform2D(c)
    }

    /// An affine transform representing uniform scaling.
    #[inline(always)]
    pub const fn scale(amount: f32) -> Transform2D {
        Transform2D([amount, 0.0, 0.0, amount, 0.0, 0.0])
    }

    #[inline(always)]
    pub const fn translation(self) -> Xy<f32> {
        Xy {
            x: self.0[4],
            y: self.0[5],
        }
    }

    pub const fn determinant(self) -> f32 {
        self.0[0] * self.0[3] - self.0[1] * self.0[2]
    }

    pub const fn inverse(self) -> Self {
        let inv_det = self.determinant().recip();
        Self([
            inv_det * self.0[3],
            -inv_det * self.0[1],
            -inv_det * self.0[2],
            inv_det * self.0[0],
            inv_det * (self.0[2] * self.0[5] - self.0[3] * self.0[4]),
            inv_det * (self.0[1] * self.0[4] - self.0[0] * self.0[5]),
        ])
    }

    pub fn transform_area(self, area: Aabb2D<f32>) -> Aabb2D<f32> {
        let p00 = self * Xy::new(area.x_min, area.y_min);
        let p01 = self * Xy::new(area.x_min, area.y_max);
        let p10 = self * Xy::new(area.x_max, area.y_min);
        let p11 = self * Xy::new(area.x_max, area.y_max);
        Aabb2D::from_min_max(p00, p01).union(Aabb2D::from_min_max(p10, p11))
    }
}

impl Mul<Xy<f32>> for Transform2D {
    type Output = Xy<f32>;

    #[inline]
    fn mul(self, other: Xy<f32>) -> Xy<f32> {
        Xy {
            x: self.0[0] * other.x + self.0[2] * other.y + self.0[4],
            y: self.0[1] * other.x + self.0[3] * other.y + self.0[5],
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub const fn cross(&self) -> Self {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }

    #[inline]
    pub fn pack_xy<V>(self, axis_value: V, cross_value: V) -> Xy<V> {
        match self {
            Self::Horizontal => Xy::new(axis_value, cross_value),
            Self::Vertical => Xy::new(cross_value, axis_value),
        }
    }
}
