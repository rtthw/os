//! # Mathematics

use std::ops::{Add, Mul, Sub};



#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct Aabb2D<V> {
    pub min: Xy<V>,
    pub max: Xy<V>,
}

impl Aabb2D<f32> {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    #[inline]
    pub const fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min: Xy::new(min_x, min_y),
            max: Xy::new(max_x, max_y),
        }
    }

    #[inline]
    pub const fn from_min_max(min: Xy<f32>, max: Xy<f32>) -> Self {
        Self { min, max }
    }

    #[inline]
    pub const fn from_size(size: Xy<f32>) -> Self {
        Self {
            min: Xy::ZERO,
            max: size,
        }
    }

    #[inline]
    pub const fn size(&self) -> Xy<f32> {
        Xy::new(self.max.x - self.min.x, self.max.y - self.min.y)
    }

    #[inline]
    pub const fn position(&self) -> Xy<f32> {
        Xy::new(self.min.x, self.min.y)
    }

    #[inline]
    pub const fn abs(&self) -> Self {
        let Self { min, max } = *self;
        Self::new(
            min.x.min(max.x),
            min.y.min(max.y),
            min.x.max(max.x),
            min.y.max(max.y),
        )
    }

    #[inline]
    pub fn set_size(&mut self, size: Xy<f32>) {
        self.max.x = self.min.x + size.x;
        self.max.y = self.min.y + size.y;
    }

    #[inline]
    pub const fn translate(&self, amount: Xy<f32>) -> Self {
        Self {
            min: self.min.const_add(amount),
            max: self.max.const_add(amount),
        }
    }

    #[inline]
    pub const fn intersect(&self, other: Self) -> Self {
        let min_x = self.min.x.max(other.min.x);
        let min_y = self.min.y.max(other.min.y);
        let max_x = self.max.x.min(other.max.x);
        let max_y = self.max.y.min(other.max.y);

        Self::new(min_x, min_y, max_x.max(min_x), max_y.max(min_y))
    }

    #[inline]
    pub const fn union(&self, other: Self) -> Self {
        Self::new(
            self.min.x.min(other.min.x),
            self.min.y.min(other.min.y),
            self.max.x.max(other.max.x),
            self.max.y.max(other.max.y),
        )
    }

    #[inline]
    pub const fn contains(&self, point: Xy<f32>) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    #[inline]
    pub const fn overlaps(&self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    #[inline]
    pub const fn add_insets(self, other: Self) -> Self {
        let other = other.abs();
        Self::new(
            other.min.x - self.min.x,
            other.min.y - self.min.y,
            other.max.x + self.max.x,
            other.max.y + self.max.y,
        )
    }
}



#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
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

    #[inline]
    pub const fn const_add(&self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
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
        let p00 = self * Xy::new(area.min.x, area.min.y);
        let p01 = self * Xy::new(area.min.x, area.max.y);
        let p10 = self * Xy::new(area.max.x, area.min.y);
        let p11 = self * Xy::new(area.max.x, area.max.y);

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
