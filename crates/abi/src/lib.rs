//! # Application Binary Interface (ABI)

pub mod elf;
pub mod layout;
pub mod path;
pub mod stable_string;
pub mod stable_vec;
pub mod tree;
pub mod type_map;

pub use {
    path::Path,
    stable_string::StableString,
    stable_vec::StableVec,
    type_map::{TypeMap, TypeMapEntry},
};

use {
    core::{
        any::Any,
        ops::{Deref, DerefMut, Sub},
    },
    std::{
        collections::HashMap,
        ops::{Add, Mul},
    },
};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");



pub trait App<U: Any> {
    fn update(&mut self, update: U) -> Result<(), &'static str>;

    /// Convert this concrete application instance into a [`WrappedApp`] for
    /// passing across ABI boundaries.
    fn wrap(self) -> Box<dyn WrappedApp>
    where
        Self: Sized + 'static,
    {
        struct Wrapper<A: App<U>, U: Any> {
            app: A,
            _update_type: core::marker::PhantomData<fn() -> U>,
        }

        impl<A: App<U>, U: Any> WrappedApp for Wrapper<A, U> {
            fn update(&mut self, update: Box<dyn Any>) -> Result<(), &'static str> {
                match update.downcast::<U>() {
                    Ok(update) => {
                        self.app.update(*update)?;
                        Ok(())
                    }
                    Err(_value) => Err("invalid type"),
                }
            }
        }

        Box::new(Wrapper {
            app: self,
            _update_type: core::marker::PhantomData,
        })
    }
}

/// Wrapper type necessary for soundly interacting with generic [`App`]
/// instances.
///
/// See [`App::wrap`] for implementation details.
pub trait WrappedApp {
    /// Pass a type-erased update to the underlying [`App`].
    fn update(&mut self, update: Box<dyn Any>) -> Result<(), &'static str>;
}

#[derive(Debug)]
pub struct Manifest {
    pub name: &'static str,
    pub init: fn() -> Box<dyn WrappedApp>,
    pub dependencies: &'static [&'static str],
    pub abi_version: &'static str,
}

#[macro_export]
macro_rules! manifest {
    (
        name: $name_def:expr,
        init: $init_def:expr,
        dependencies: $dependencies_def:expr,
    ) => {
        #[unsafe(no_mangle)]
        pub static __MANIFEST: $crate::Manifest = $crate::Manifest {
            name: $name_def,
            init: $init_def,
            dependencies: $dependencies_def,
            abi_version: $crate::VERSION,
        };
    };
}

#[macro_export]
macro_rules! include {
    (
        mod $name:ident {
            $(
                fn $fn_name:ident ($( $fn_arg:ident : $fn_arg_ty:ty ),*) $( -> $fn_ret_ty:ty )?;
            )*
        }
    ) => {
        pub mod $name {
            unsafe extern "Rust" {
                $(
                    #[link_name = concat!("__", stringify!($name), "_", stringify!($fn_name))]
                    pub fn $fn_name($($fn_arg: $fn_arg_ty)*) $(-> $fn_ret_ty)? ;
                )*
            }
        }
    };
}

#[macro_export]
macro_rules! declare {
    (
        mod $name:ident {
            $(
                fn $fn_name:ident ($( $fn_arg:ident : $fn_arg_ty:ty ),*) $( -> $fn_ret_ty:ty )? {
                    $( $fn_body:tt )*
                }
            )*
        }
    ) => {
        $(
            #[unsafe(export_name = concat!("__", stringify!($name), "_", stringify!($fn_name)))]
            pub unsafe extern "Rust" fn $fn_name($($fn_arg: $fn_arg_ty)*) $(-> $fn_ret_ty)? {
                $($fn_body)*
            }
        )*
    };
}



#[derive(Clone, Copy, Debug, Default)]
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

    pub const fn from_min_max(min: Xy<f32>, max: Xy<f32>) -> Self {
        Self {
            x_min: min.x,
            x_max: max.x,
            y_min: min.y,
            y_max: max.y,
        }
    }

    pub const fn from_size(size: Xy<f32>) -> Self {
        Self {
            x_min: 0.0,
            x_max: size.x,
            y_min: 0.0,
            y_max: size.y,
        }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum Length {
    Fill,
    Portion(u16),
    Shrink,
    Exact(f32),
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Rgba<V> {
    pub r: V,
    pub g: V,
    pub b: V,
    pub a: V,
}



#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum InputEvent {
    MouseButtonDown(MouseButton),
    MouseButtonUp(MouseButton),
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum MouseButton {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}



pub struct ViewSettings {
    pub double_click_timeout: f64,
}

pub enum ViewEvent {
    MouseMove { delta: Xy<f32> },
    MouseEnter { node: u64 },
    MouseLeave { node: u64 },
    Click { node: u64 },
    RightClick { node: u64 },
    DoubleClick { node: u64 },
    Focus { old: Option<u64>, new: u64 },
}

pub trait Element {
    fn children_ids(&self) -> Vec<u64> {
        Vec::new()
    }

    fn clickable(&self) -> bool {
        false
    }

    fn focusable(&self) -> bool {
        false
    }

    fn render(&mut self, pass: &mut RenderPass<'_>);

    fn render_overlay(&mut self, pass: &mut RenderPass<'_>);
}

pub struct ElementInfo {
    pub element: Box<dyn Element>,
    pub state: ElementState,
}

impl Deref for ElementInfo {
    type Target = dyn Element;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.element
    }
}

impl DerefMut for ElementInfo {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.element
    }
}

pub struct ElementState {
    pub id: u64,
    pub bounds: Aabb2D<f32>,

    pub local_transform: Transform2D,
    pub global_transform: Transform2D,

    pub needs_render: bool,
    pub wants_render: bool,
    pub wants_overlay_render: bool,
}

impl ElementState {
    fn new(id: u64) -> Self {
        Self {
            id,
            bounds: Aabb2D::ZERO,
            local_transform: Transform2D::IDENTITY,
            global_transform: Transform2D::IDENTITY,
            needs_render: true,
            wants_render: true,
            wants_overlay_render: true,
        }
    }

    fn merge_with_child(&mut self, child_state: &Self) {
        self.needs_render |= child_state.needs_render;
    }
}



#[derive(Default)]
pub struct Render {
    pub quads: Vec<RenderQuad>,
    pub texts: Vec<RenderText>,
}

#[derive(Clone)]
pub struct RenderQuad {
    pub bounds: Aabb2D<f32>,
    pub color: Rgba<u8>,
}

#[derive(Clone)]
pub struct RenderText {
    pub content: String,
    pub bounds: Aabb2D<f32>,
    pub color: Rgba<u8>,
}

impl Render {
    pub fn extend(&mut self, other: &Render, transform: Transform2D) {
        self.quads
            .extend(other.quads.iter().cloned().map(|quad| RenderQuad {
                bounds: transform.transform_area(quad.bounds),
                ..quad
            }));
        self.texts
            .extend(other.texts.iter().cloned().map(|text| RenderText {
                bounds: transform.transform_area(text.bounds),
                ..text
            }));
    }
}

pub struct RenderPass<'view> {
    state: &'view mut ElementState,
    render: &'view mut Render,
}

impl RenderPass<'_> {
    pub fn bounds(&self) -> Aabb2D<f32> {
        self.state.bounds
    }

    pub fn fill_quad(&mut self, bounds: Aabb2D<f32>, color: Rgba<u8>) {
        self.render.quads.push(RenderQuad { bounds, color });
    }

    pub fn fill_text(&mut self, content: impl Into<String>, bounds: Aabb2D<f32>, color: Rgba<u8>) {
        self.render.texts.push(RenderText {
            content: content.into(),
            bounds,
            color,
        });
    }
}

pub fn render_pass(
    root_node: tree::NodeMut<'_, ElementInfo>,
    render_cache: &mut HashMap<u64, (Render, Render)>,
) -> Render {
    let mut final_render = Render::default();

    render_element(root_node, render_cache, &mut final_render);

    final_render
}

pub fn render_element(
    node: tree::NodeMut<'_, ElementInfo>,
    render_cache: &mut HashMap<u64, (Render, Render)>,
    final_render: &mut Render,
) {
    let children = node.leaves;
    let element = &mut *node.element.element;
    let state = &mut node.element.state;

    if state.wants_render || state.wants_overlay_render {
        let (render, overlay_render) = render_cache.entry(state.id).or_default();

        if state.wants_render {
            let mut pass = RenderPass { state, render };
            element.render(&mut pass);
        }
        if state.wants_overlay_render {
            let mut pass = RenderPass {
                state,
                render: overlay_render,
            };
            element.render_overlay(&mut pass);
        }
    }

    state.needs_render = false;
    state.wants_render = false;
    state.wants_overlay_render = false;

    {
        let transform = state.global_transform;
        let Some((render, _)) = &mut render_cache.get(&state.id) else {
            return;
        };

        final_render.extend(render, transform);
    }

    let parent_state = &mut *state;
    for_each_child_element(element, children, |mut node| {
        render_element(node.reborrow_mut(), render_cache, final_render);
        parent_state.merge_with_child(&node.element.state);
    });

    {
        let transform = state.global_transform;
        let Some((_, overlay_render)) = &mut render_cache.get(&state.id) else {
            return;
        };

        final_render.extend(overlay_render, transform);
    }
}



fn for_each_child_element(
    element: &mut dyn Element,
    mut children: tree::LeavesMut<'_, ElementInfo>,
    mut callback: impl FnMut(tree::NodeMut<'_, ElementInfo>),
) {
    for child_id in element.children_ids() {
        callback(
            children
                .get_mut(child_id)
                .expect("Element::children_ids produced an invalid child ID"),
        );
    }
}
