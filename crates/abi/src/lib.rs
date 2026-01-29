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
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
    },
};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");



// TODO: Use the `Element` trait instead of this one?
pub trait App<U: Any> {
    fn view(&mut self) -> impl Element + 'static;

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
            fn build_view(&mut self, fonts: Box<dyn Fonts>, window_size: Xy<f32>) -> View {
                View {
                    fonts,
                    tree: tree::Tree::new(),
                    root_element: ElementBuilder::new(self.app.view()).into_child(),
                    window_size,
                }
            }

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
    fn build_view(&mut self, fonts: Box<dyn Fonts>, window_size: Xy<f32>) -> View;
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

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum Length {
    MaxContent,
    MinContent,
    FitContent(f32),
    Exact(f32),
}

impl Length {
    pub const fn exact(&self) -> Option<f32> {
        if let Self::Exact(amount) = *self {
            Some(amount)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum LengthRequest {
    MaxContent,
    MinContent,
    FitContent(f32),
}

impl Into<Length> for LengthRequest {
    fn into(self) -> Length {
        match self {
            LengthRequest::MaxContent => Length::MaxContent,
            LengthRequest::MinContent => Length::MinContent,
            LengthRequest::FitContent(max_size) => Length::FitContent(max_size),
        }
    }
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



#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum TextWrapMode {
    #[default]
    Wrap = 0,
    NoWrap = 1,
}

/// How text content is aligned within a container.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum TextAlignment {
    /// Align text content to the beginning edge.
    ///
    /// This is equivalent to [`TextAlignment::Left`] for left-to-right text,
    /// and [`TextAlignment::Right`] for right-to-left text.
    #[default]
    Start = 0,
    /// Align text content to the ending edge.
    ///
    /// This is equivalent to [`TextAlignment::Right`] for left-to-right text,
    /// and [`TextAlignment::Left`] for right-to-left text.
    End = 1,
    /// Align text content to the left edge.
    Left = 2,
    /// Align text content to the center.
    Center = 3,
    /// Align text content to the right edge.
    Right = 4,
    /// Justify text content to fill all available space, with the last line
    /// unaffected.
    Justify = 5,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    Relative(f32),
    Absolute(f32),
}

impl Default for LineHeight {
    fn default() -> Self {
        Self::FONT_PREFERRED
    }
}

impl LineHeight {
    pub const FONT_PREFERRED: Self = Self::Relative(1.0);
}



pub struct View {
    fonts: Box<dyn Fonts>,
    tree: tree::Tree<ElementInfo>,
    root_element: ChildElement,
    window_size: Xy<f32>,
}

pub trait Fonts {
    fn measure_text(
        &mut self,
        id: u64,
        text: &Arc<str>,
        max_advance: Option<f32>,
        font_size: f32,
        line_height: LineHeight,
        font_style: FontStyle,
        alignment: TextAlignment,
        wrap_mode: TextWrapMode,
    ) -> Xy<f32>;
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

    #[allow(unused)]
    fn update_children(&mut self, pass: &mut UpdatePass<'_>) {}

    fn clickable(&self) -> bool {
        false
    }

    fn focusable(&self) -> bool {
        false
    }

    #[allow(unused)]
    fn render(&mut self, pass: &mut RenderPass<'_>) {}

    #[allow(unused)]
    fn render_overlay(&mut self, pass: &mut RenderPass<'_>) {}

    fn layout(&mut self, pass: &mut LayoutPass<'_>);

    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32;

    /// Called when this element is added to the view tree.
    #[allow(unused)]
    fn on_build(&mut self, pass: &mut UpdatePass<'_>) {}
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

    pub layout_size: Xy<f32>,

    pub local_transform: Transform2D,
    pub global_transform: Transform2D,

    pub newly_added: bool,
    pub children_changed: bool,

    pub needs_render: bool,
    pub wants_render: bool,
    pub wants_overlay_render: bool,

    pub needs_layout: bool,
    pub wants_layout: bool,
    pub moved: bool,
}

impl ElementState {
    /// See [`UpdatePass::update_child`].
    fn new(id: u64) -> Self {
        Self {
            id,
            bounds: Aabb2D::ZERO,
            layout_size: Xy::ZERO,
            local_transform: Transform2D::IDENTITY,
            global_transform: Transform2D::IDENTITY,
            newly_added: true,
            children_changed: true,
            needs_render: true,
            wants_render: true,
            wants_overlay_render: true,
            needs_layout: true,
            wants_layout: true,
            moved: true,
        }
    }

    fn merge_with_child(&mut self, child_state: &Self) {
        self.children_changed |= child_state.children_changed;
        self.needs_render |= child_state.needs_render;
        self.needs_layout |= child_state.needs_layout;
    }
}

pub struct ElementProperties {
    pub width: Option<Length>,
    pub height: Option<Length>,
}

pub struct ElementBuilder {
    id: u64,
    element: Box<dyn Element>,
}

impl ElementBuilder {
    pub fn new<E: Element + 'static>(element: E) -> Self {
        static NEXT_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);

        Self {
            id,
            element: Box::new(element),
        }
    }

    pub fn into_child(self) -> ChildElement {
        ChildElement {
            id: self.id,
            inner: ChildElementInner::New(self),
        }
    }
}

pub struct ChildElement {
    id: u64,
    inner: ChildElementInner,
}

impl ChildElement {
    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }

    fn exists(&self) -> bool {
        matches!(self.inner, ChildElementInner::Existing)
    }

    fn take_inner(&mut self) -> Option<ElementBuilder> {
        match std::mem::replace(&mut self.inner, ChildElementInner::Existing) {
            ChildElementInner::New(builder) => Some(builder),
            ChildElementInner::Existing => None,
        }
    }
}

enum ChildElementInner {
    Existing,
    New(ElementBuilder),
}



pub struct Column {
    children: Vec<ChildElement>,
}

impl Column {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn with(mut self, child: impl Element + 'static) -> Self {
        self.children.push(ElementBuilder::new(child).into_child());
        self
    }
}

impl Element for Column {
    fn children_ids(&self) -> Vec<u64> {
        self.children.iter().map(|child| child.id()).collect()
    }

    fn update_children(&mut self, pass: &mut UpdatePass<'_>) {
        for child in self.children.iter_mut() {
            pass.update_child(child);
        }
    }

    fn layout(&mut self, pass: &mut LayoutPass<'_>) {
        let width = Length::FitContent(pass.size.x);
        let height = Length::FitContent(pass.size.y);
        let auto_size = Xy::new(width, height);

        let mut y_offset = 0.0;
        for child in &mut self.children {
            let child_size = pass.resolve_size(child.id(), auto_size);
            pass.do_layout(child, child_size);
            pass.place_child(child, Xy::new(0.0, y_offset));

            y_offset += child_size.y;
        }
    }

    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32 {
        let (length_request, min_result) = match length_request {
            LengthRequest::MinContent | LengthRequest::MaxContent => (length_request, 0.0),
            LengthRequest::FitContent(space) => (LengthRequest::MinContent, space),
        };

        let fallback_length = length_request.into();

        let mut length: f32 = 0.0;
        for child in &mut self.children {
            let child_length =
                context.resolve_length(child.id(), axis, fallback_length, cross_length);
            match axis {
                Axis::Horizontal => length = length.max(child_length),
                Axis::Vertical => length += child_length,
            }
        }

        min_result.max(length)
    }
}



pub struct Label {
    pub text: Arc<str>,
    pub font_size: f32,
    pub line_height: LineHeight,
    pub font_style: FontStyle,
    pub alignment: TextAlignment,
    pub wrap_mode: TextWrapMode,
}

impl Label {
    pub fn new(text: impl Into<Arc<str>>) -> Self {
        Self {
            text: text.into(),
            font_size: 16.0,
            line_height: LineHeight::FONT_PREFERRED,
            font_style: FontStyle::Normal,
            alignment: TextAlignment::Start,
            wrap_mode: TextWrapMode::Wrap,
        }
    }
}

impl Element for Label {
    fn children_ids(&self) -> Vec<u64> {
        unsafe { __ui_Label__children_ids(self) }
    }

    fn render(&mut self, pass: &mut RenderPass<'_>) {
        unsafe { __ui_Label__render(self, pass) }
    }

    fn layout(&mut self, pass: &mut LayoutPass<'_>) {
        unsafe { __ui_Label__layout(self, pass) }
    }

    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32 {
        unsafe { __ui_Label__measure(self, context, axis, length_request, cross_length) }
    }
}

unsafe extern "Rust" {
    fn __ui_Label__children_ids(label: &Label) -> Vec<u64>;

    fn __ui_Label__render(label: &mut Label, pass: &mut RenderPass<'_>);

    fn __ui_Label__layout(label: &mut Label, pass: &mut LayoutPass<'_>);

    fn __ui_Label__measure(
        label: &mut Label,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32;
}



pub struct UpdatePass<'view> {
    state: &'view mut ElementState,
    children: tree::LeavesMut<'view, ElementInfo>,
}

impl UpdatePass<'_> {
    /// See [`Element::update_children`].
    pub fn update_child(&mut self, child: &mut ChildElement) {
        let Some(ElementBuilder { id, element }) = child.take_inner() else {
            return;
        };

        let state = ElementState::new(id);
        let info = ElementInfo { element, state };

        self.children.insert(id, info);
    }
}

pub fn update_pass(view: &mut View) {
    let mut node = view
        .tree
        .find_mut(view.root_element.id())
        .expect("failed to find the view's root node");

    {
        let children = node.leaves.reborrow_mut();
        let state = &mut node.element.state;

        if !view.root_element.exists() {
            UpdatePass { state, children }.update_child(&mut view.root_element);
        }
    }

    update_element_tree(node);
}

fn update_element_tree(node: tree::NodeMut<'_, ElementInfo>) {
    let mut children = node.leaves;
    let element = &mut *node.element.element;
    let state = &mut node.element.state;

    if !state.children_changed {
        return;
    }

    state.children_changed = false;

    element.update_children(&mut UpdatePass {
        state,
        children: children.reborrow_mut(),
    });

    if state.newly_added {
        state.newly_added = false;
        element.on_build(&mut UpdatePass {
            state,
            children: children.reborrow_mut(),
        });
    }

    let parent_state = &mut *state;
    for_each_child_element(element, children, |mut node| {
        update_element_tree(node.reborrow_mut());
        parent_state.merge_with_child(&node.element.state);
    });
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
    pub content: Arc<str>,
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

    pub fn fill_text(
        &mut self,
        content: impl Into<Arc<str>>,
        bounds: Aabb2D<f32>,
        color: Rgba<u8>,
    ) {
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



pub struct LayoutPass<'view> {
    fonts: &'view mut dyn Fonts,
    state: &'view mut ElementState,
    children: tree::LeavesMut<'view, ElementInfo>,
    size: Xy<f32>,
}

impl LayoutPass<'_> {
    #[inline]
    pub fn fonts(&self) -> &dyn Fonts {
        self.fonts
    }

    #[inline]
    pub fn fonts_mut(&mut self) -> &mut dyn Fonts {
        self.fonts
    }

    pub fn do_layout(&mut self, child: &mut ChildElement, size: Xy<f32>) {
        let mut node = self
            .children
            .get_mut(child.id)
            .expect("invalid child passed to LayoutPass::do_layout");
        layout_element(self.fonts, node.reborrow_mut(), size);
        self.state.merge_with_child(&node.element.state);
    }

    pub fn place_child(&mut self, child: &mut ChildElement, position: Xy<f32>) {
        move_element(
            &mut self
                .children
                .get_mut(child.id)
                .expect("invalid child passed to LayoutPass::place_child")
                .element
                .state,
            position,
        );
    }

    pub fn resolve_size(&mut self, child_id: u64, fallback_size: Xy<Length>) -> Xy<f32> {
        let node = self
            .children
            .get_mut(child_id)
            .expect("provided invalid child ID to LayoutPass::resolve_size");

        resolve_element_size(self.fonts, node, fallback_size) // , self.size)
    }
}

pub fn layout_pass(view: &mut View) {
    let node = view
        .tree
        .find_mut(view.root_element.id())
        .expect("failed to find the view's root node");
    layout_element(&mut *view.fonts, node, view.window_size);
}

pub fn layout_element(fonts: &mut dyn Fonts, node: tree::NodeMut<'_, ElementInfo>, size: Xy<f32>) {
    let element = &mut *node.element.element;
    let state = &mut node.element.state;
    let children = node.leaves;

    let mut pass = LayoutPass {
        fonts,
        state,
        children,
        size,
    };
    element.layout(&mut pass);
}

fn move_element(state: &mut ElementState, position: Xy<f32>) {
    let end_point = position + state.layout_size;

    let position = position.round();
    let end_point = end_point.round();

    if position.x != state.bounds.x_min || position.y != state.bounds.y_min {
        state.moved = true;
    }

    state.bounds.x_min = position.x;
    state.bounds.y_min = position.y;
    state.bounds.x_max = end_point.x;
    state.bounds.y_max = end_point.y;
}

pub struct MeasureContext<'pass> {
    fonts: &'pass mut dyn Fonts,
    state: &'pass mut ElementState,
    children: tree::LeavesMut<'pass, ElementInfo>,
}

impl MeasureContext<'_> {
    #[inline]
    pub fn fonts(&self) -> &dyn Fonts {
        self.fonts
    }

    #[inline]
    pub fn fonts_mut(&mut self) -> &mut dyn Fonts {
        self.fonts
    }

    // TODO: Don't just default to the fallback here. Get something from the child
    //       state?
    pub fn resolve_length(
        &mut self,
        child_id: u64,
        axis: Axis,
        fallback_length: Length,
        cross_length: Option<f32>,
    ) -> f32 {
        let child = self
            .children
            .get_mut(child_id)
            .expect("invalid child ID provided to MeasureContext::resolve_length");
        let element = &mut *child.element.element;
        let state = &mut child.element.state;
        let children = child.leaves;

        let mut context = MeasureContext {
            fonts: self.fonts,
            state,
            children,
        };

        fallback_length.exact().unwrap_or_else(|| {
            resolve_axis_measurement(&mut context, element, axis, fallback_length, cross_length)
        })
    }
}

// TODO: Don't just default to the fallback here. Get something from the child
//       state?
fn resolve_element_size(
    fonts: &mut dyn Fonts,
    node: tree::NodeMut<'_, ElementInfo>,
    fallback_size: Xy<Length>,
) -> Xy<f32> {
    let element = &mut *node.element.element;
    let state = &mut node.element.state;
    let children = node.leaves;

    // TODO: Consider supporting different inline/block axes?

    let inline_axis = Axis::Horizontal;
    let block_axis = Axis::Vertical;

    let inline_length = fallback_size.x;
    let block_length = fallback_size.y;

    let inline_measurement = inline_length.exact();
    let block_measurement = block_length.exact();

    // Early return.
    if let Some(x) = inline_measurement
        && let Some(y) = block_measurement
    {
        return Xy::new(x, y);
    }

    let mut context = MeasureContext {
        fonts,
        state,
        children,
    };

    let inline_measurement = inline_measurement.unwrap_or_else(|| {
        resolve_axis_measurement(
            &mut context,
            element,
            inline_axis,
            inline_length,
            block_measurement,
        )
    });

    let block_measurement = block_measurement.unwrap_or_else(|| {
        resolve_axis_measurement(
            &mut context,
            element,
            block_axis,
            block_length,
            Some(inline_measurement),
        )
    });

    Xy::new(inline_measurement, block_measurement)
}

fn resolve_axis_measurement(
    context: &mut MeasureContext<'_>,
    element: &mut dyn Element,
    axis: Axis,
    length: Length,
    cross_length: Option<f32>,
) -> f32 {
    let length_request = match length {
        Length::MaxContent => LengthRequest::MaxContent,
        Length::MinContent => LengthRequest::MinContent,
        Length::FitContent(max_size) => LengthRequest::FitContent(max_size),
        Length::Exact(amount) => return amount,
    };
    element.measure(context, axis, length_request, cross_length)
}



macro_rules! multi_impl {
    ($ty:ty, { $($item:item)+ }) => {
        impl $ty { $($item)+ }
    };
    ($ty:ty, $($others:ty),+, { $($item:item)+ }) => {
        multi_impl!($ty, { $($item)+ });
        multi_impl!($($others),+, { $($item)+ });
    };
}

// Types with a `state: &mut ElementState` field.
multi_impl! {
    LayoutPass<'_>,
    MeasureContext<'_>,
    RenderPass<'_>,
    UpdatePass<'_>,
    {
        pub fn id(&self) -> u64 {
            self.state.id
        }

        pub fn request_render(&mut self) {
            self.state.wants_render = true;
        }

        pub fn request_layout(&mut self) {
            self.state.wants_layout = true;
        }
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
