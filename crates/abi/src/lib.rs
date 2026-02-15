//! # Application Binary Interface (ABI)

pub mod cursor_icon;
pub mod elf;
pub mod flex;
pub mod layout;
pub mod math;
pub mod path;
pub mod stable_string;
pub mod stable_vec;
pub mod text;
pub mod tree;
pub mod type_map;

pub use {
    cursor_icon::CursorIcon,
    flex::{AxisAlignment, CrossAlignment, Flex, FlexParams},
    math::{Aabb2D, Axis, Transform2D, Xy},
    path::Path,
    stable_string::StableString,
    stable_vec::StableVec,
    text::{FontStyle, LineHeight, TextAlignment, TextWrapMode},
    type_map::{TypeMap, TypeMapEntry},
};

use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");



#[derive(Debug)]
pub struct Manifest {
    pub name: &'static str,
    pub init: fn() -> ElementBuilder,
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



#[repr(C)]
pub struct DriverInput {
    pub id: u64,
    pub known_bounds: Aabb2D<f32>,
    pub events: [Option<DriverInputEvent>; DRIVER_INPUT_EVENT_CAPACITY],
    pub render: Render,
}

pub const DRIVER_INPUT_EVENT_CAPACITY: usize = 16;

impl DriverInput {
    pub fn new(initial_bounds: Aabb2D<f32>) -> Self {
        Self {
            id: 0,
            known_bounds: initial_bounds,
            events: [None; DRIVER_INPUT_EVENT_CAPACITY],
            render: Render::default(),
        }
    }

    pub fn push_event(&mut self, event: DriverInputEvent) -> Option<DriverInputEvent> {
        if let Some(null_index) = self.events.iter().position(|event| event.is_none()) {
            self.events[null_index] = Some(event);
            None
        } else {
            let missed_event = self.events[0].take();
            self.events.rotate_left(1);
            self.events[DRIVER_INPUT_EVENT_CAPACITY - 1] = Some(event);
            missed_event
        }
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = DriverInputEvent> {
        self.events.iter_mut().flat_map(|event| event.take())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C, u32)]
pub enum DriverInputEvent {
    Pointer(PointerEvent),
    Other(u32),
    WindowResize(Aabb2D<f32>),
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

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Rgba<V> {
    pub r: V,
    pub g: V,
    pub b: V,
    pub a: V,
}

impl Rgba<u8> {
    pub const NONE: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub const WHITE: Self = Self::rgb(0xff, 0xff, 0xff);
    pub const BLACK: Self = Self::rgb(0x00, 0x00, 0x00);

    #[inline]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 0xff }
    }

    #[inline]
    pub const fn with_red(mut self, r: u8) -> Self {
        self.r = r;
        self
    }

    #[inline]
    pub const fn with_green(mut self, g: u8) -> Self {
        self.g = g;
        self
    }

    #[inline]
    pub const fn with_blue(mut self, b: u8) -> Self {
        self.b = b;
        self
    }

    #[inline]
    pub const fn with_alpha(mut self, a: u8) -> Self {
        self.a = a;
        self
    }
}



pub struct View {
    fonts: Box<dyn Fonts>,
    tree: tree::Tree<ElementInfo>,
    root_element_id: u64,
    window_size: Xy<f32>,
    render_cache: HashMap<u64, (CachedRender, CachedRender)>,
    pointer_position: Option<Xy<f32>>,
    pointer_capture_target: Option<u64>,
    hovered_path: Vec<u64>,
    cursor_icon: CursorIcon,
    focused_element: Option<u64>,
    next_focused_element: Option<u64>,
    focused_path: Vec<u64>,
    last_animation: Option<Instant>,
}

impl View {
    pub fn new(root_builder: ElementBuilder, fonts: Box<dyn Fonts>, window_size: Xy<f32>) -> Self {
        let mut tree = tree::Tree::new();

        let Some(ElementBuilder { id, element }) = root_builder.into_child().take_inner() else {
            unreachable!();
        };

        let state = ElementState::new(id);
        let info = ElementInfo { element, state };

        tree.roots_mut().insert(id, info);

        let mut this = Self {
            fonts,
            tree,
            root_element_id: id,
            window_size,
            render_cache: HashMap::new(),
            pointer_position: None,
            pointer_capture_target: None,
            hovered_path: Vec::new(),
            cursor_icon: CursorIcon::Default,
            focused_element: None,
            next_focused_element: None,
            focused_path: Vec::new(),
            last_animation: None,
        };

        update_pass(&mut this);
        layout_pass(&mut this);
        compose_pass(&mut this);

        this
    }

    #[inline]
    pub fn cursor_icon(&self) -> CursorIcon {
        self.cursor_icon
    }

    pub fn animating(&self) -> bool {
        self.tree
            .roots()
            .get(self.root_element_id)
            .expect("infallible")
            .element
            .state
            .needs_animate
    }

    pub fn resize_window(&mut self, size: Xy<f32>) {
        if self.window_size == size {
            return;
        }
        self.window_size = size;

        layout_pass(self);
    }

    pub fn render(&mut self, render: &mut Render) {
        let now = Instant::now();
        let last = self.last_animation.take();
        let elapsed = last.map(|t| now.duration_since(t)).unwrap_or_default();

        animation_pass(self, elapsed.as_secs_f64());

        self.last_animation = self.animating().then_some(now);

        render_pass(self, render);
    }

    pub fn handle_keyboard_event(&mut self, event: KeyboardEvent) {
        keyboard_event_pass(self, &event);
        layout_pass(self);
        compose_pass(self);
    }

    pub fn handle_pointer_event(&mut self, event: PointerEvent) {
        pointer_event_pass(self, &event);
        update_pointer_pass(self);
        update_focus_pass(self);
        layout_pass(self);
        compose_pass(self);
    }
}

pub trait Fonts {
    fn measure_text(
        &mut self,
        id: u64,
        text: &str,
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

pub trait Element: Any {
    fn children_ids(&self) -> Vec<u64> {
        Vec::new()
    }

    #[allow(unused)]
    fn update_children(&mut self, pass: &mut UpdatePass<'_>) {}

    /// Defaults to `true`.
    fn accepts_pointer_events(&self) -> bool {
        true
    }

    /// Defaults to `false`.
    fn accepts_keyboard_events(&self) -> bool {
        false
    }

    /// Defaults to `false`.
    fn accepts_focus_events(&self) -> bool {
        false
    }

    #[allow(unused)]
    fn render(&mut self, pass: &mut RenderPass<'_>) {}

    #[allow(unused)]
    fn render_overlay(&mut self, pass: &mut RenderPass<'_>) {}

    #[allow(unused)]
    fn animate(&mut self, pass: &mut AnimatePass<'_>, dt: f64) {}

    fn layout(&mut self, pass: &mut LayoutPass<'_>);

    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32;

    #[allow(unused)]
    fn compose(&mut self, pass: &mut ComposePass<'_>) {}

    fn cursor_icon(&self) -> CursorIcon {
        CursorIcon::Default
    }

    /// Called when this element is added to the view tree.
    #[allow(unused)]
    fn on_build(&mut self, pass: &mut UpdatePass<'_>) {}

    /// Called when this element is interacted with by the user's keyboard.
    #[allow(unused)]
    fn on_keyboard_event(&mut self, pass: &mut EventPass<'_>, event: &KeyboardEvent) {}

    /// Called when this element is interacted with by the user's pointer.
    #[allow(unused)]
    fn on_pointer_event(&mut self, pass: &mut EventPass<'_>, event: &PointerEvent) {}

    #[allow(unused)]
    fn on_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {}

    #[allow(unused)]
    fn on_child_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {}

    #[allow(unused)]
    fn on_focus(&mut self, pass: &mut EventPass<'_>, focused: bool) {}

    #[allow(unused)]
    fn on_child_focus(&mut self, pass: &mut EventPass<'_>, focused: bool) {}
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
    pub baseline_offset: f32,
    pub layout_bounds: Aabb2D<f32>,
    pub layout_baseline_offset: f32,

    pub scroll_translation: Xy<f32>,
    pub local_transform: Transform2D,
    pub global_transform: Transform2D,

    pub newly_added: bool,
    pub children_changed: bool,

    pub needs_render: bool,
    pub wants_render: bool,
    pub wants_overlay_render: bool,

    pub needs_animate: bool,
    pub wants_animate: bool,

    pub needs_layout: bool,
    pub wants_layout: bool,

    pub needs_compose: bool,
    pub wants_compose: bool,
    pub transformed: bool,

    pub hovered: bool,
    pub focused: bool,
}

impl ElementState {
    /// See [`UpdatePass::update_child`].
    fn new(id: u64) -> Self {
        Self {
            id,
            bounds: Aabb2D::ZERO,
            baseline_offset: 0.0,
            layout_bounds: Aabb2D::ZERO,
            layout_baseline_offset: 0.0,
            scroll_translation: Xy::ZERO,
            local_transform: Transform2D::IDENTITY,
            global_transform: Transform2D::IDENTITY,
            newly_added: true,
            children_changed: true,
            needs_render: true,
            wants_render: true,
            wants_overlay_render: true,
            needs_animate: true,
            wants_animate: true,
            needs_layout: true,
            wants_layout: true,
            needs_compose: true,
            wants_compose: true,
            transformed: true,
            hovered: false,
            focused: false,
        }
    }

    fn merge_with_child(&mut self, child_state: &Self) {
        self.children_changed |= child_state.children_changed;
        self.needs_render |= child_state.needs_render;
        self.needs_animate |= child_state.needs_animate;
        self.needs_layout |= child_state.needs_layout;
        self.needs_compose |= child_state.needs_compose;
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

    pub fn exists(&self) -> bool {
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

pub struct TypedChildElement<E: Element> {
    pub inner: ChildElement,
    _type: PhantomData<E>,
}

impl<E: Element + 'static> TypedChildElement<E> {
    pub fn new(element: E) -> Self {
        Self {
            inner: ElementBuilder::new(element).into_child(),
            _type: PhantomData,
        }
    }

    #[inline]
    pub fn id(&self) -> u64 {
        self.inner.id
    }
}

pub trait ExtensionElement {
    fn element(&self) -> &dyn Element;
    fn element_mut(&mut self) -> &mut dyn Element;

    #[inline(always)]
    fn children_ids(&self) -> Vec<u64> {
        self.element().children_ids()
    }

    #[inline(always)]
    fn update_children(&mut self, pass: &mut UpdatePass<'_>) {
        self.element_mut().update_children(pass)
    }

    #[inline(always)]
    fn accepts_pointer_events(&self) -> bool {
        self.element().accepts_pointer_events()
    }

    #[inline(always)]
    fn accepts_keyboard_events(&self) -> bool {
        self.element().accepts_keyboard_events()
    }

    #[inline(always)]
    fn accepts_focus_events(&self) -> bool {
        self.element().accepts_focus_events()
    }

    #[inline(always)]
    fn render(&mut self, pass: &mut RenderPass<'_>) {
        self.element_mut().render(pass)
    }

    #[inline(always)]
    fn render_overlay(&mut self, pass: &mut RenderPass<'_>) {
        self.element_mut().render_overlay(pass)
    }

    #[inline(always)]
    fn animate(&mut self, pass: &mut AnimatePass<'_>, dt: f64) {
        self.element_mut().animate(pass, dt)
    }

    #[inline(always)]
    fn layout(&mut self, pass: &mut LayoutPass<'_>) {
        self.element_mut().layout(pass)
    }

    #[inline(always)]
    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32 {
        self.element_mut()
            .measure(context, axis, length_request, cross_length)
    }

    #[inline(always)]
    fn compose(&mut self, pass: &mut ComposePass<'_>) {
        self.element_mut().compose(pass)
    }

    #[inline(always)]
    fn cursor_icon(&self) -> CursorIcon {
        self.element().cursor_icon()
    }

    #[inline(always)]
    fn on_build(&mut self, pass: &mut UpdatePass<'_>) {
        self.element_mut().on_build(pass);
    }

    #[inline(always)]
    fn on_keyboard_event(&mut self, pass: &mut EventPass<'_>, event: &KeyboardEvent) {
        self.element_mut().on_keyboard_event(pass, event)
    }

    #[inline(always)]
    fn on_pointer_event(&mut self, pass: &mut EventPass<'_>, event: &PointerEvent) {
        self.element_mut().on_pointer_event(pass, event)
    }

    #[inline(always)]
    fn on_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {
        self.element_mut().on_hover(pass, hovered)
    }

    #[inline(always)]
    fn on_child_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {
        self.element_mut().on_child_hover(pass, hovered)
    }

    #[inline(always)]
    fn on_focus(&mut self, pass: &mut EventPass<'_>, focused: bool) {
        self.element_mut().on_focus(pass, focused)
    }

    #[inline(always)]
    fn on_child_focus(&mut self, pass: &mut EventPass<'_>, focused: bool) {
        self.element_mut().on_child_focus(pass, focused)
    }
}

impl<T: ExtensionElement + 'static> Element for T {
    #[inline(always)]
    fn children_ids(&self) -> Vec<u64> {
        self.children_ids()
    }

    #[inline(always)]
    fn update_children(&mut self, pass: &mut UpdatePass<'_>) {
        self.update_children(pass)
    }

    #[inline(always)]
    fn accepts_pointer_events(&self) -> bool {
        self.accepts_pointer_events()
    }

    #[inline(always)]
    fn accepts_keyboard_events(&self) -> bool {
        self.accepts_keyboard_events()
    }

    #[inline(always)]
    fn accepts_focus_events(&self) -> bool {
        self.accepts_focus_events()
    }

    #[inline(always)]
    fn render(&mut self, pass: &mut RenderPass<'_>) {
        self.render(pass)
    }

    #[inline(always)]
    fn render_overlay(&mut self, pass: &mut RenderPass<'_>) {
        self.render_overlay(pass)
    }

    #[inline(always)]
    fn animate(&mut self, pass: &mut AnimatePass<'_>, dt: f64) {
        self.animate(pass, dt)
    }

    #[inline(always)]
    fn layout(&mut self, pass: &mut LayoutPass<'_>) {
        self.layout(pass)
    }

    #[inline(always)]
    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32 {
        self.measure(context, axis, length_request, cross_length)
    }

    #[inline(always)]
    fn compose(&mut self, pass: &mut ComposePass<'_>) {
        self.compose(pass)
    }

    #[inline(always)]
    fn cursor_icon(&self) -> CursorIcon {
        self.cursor_icon()
    }

    #[inline(always)]
    fn on_build(&mut self, pass: &mut UpdatePass<'_>) {
        self.on_build(pass);
    }

    #[inline(always)]
    fn on_keyboard_event(&mut self, pass: &mut EventPass<'_>, event: &KeyboardEvent) {
        self.on_keyboard_event(pass, event)
    }

    #[inline(always)]
    fn on_pointer_event(&mut self, pass: &mut EventPass<'_>, event: &PointerEvent) {
        self.on_pointer_event(pass, event)
    }

    #[inline(always)]
    fn on_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {
        self.on_hover(pass, hovered)
    }

    #[inline(always)]
    fn on_child_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {
        self.on_child_hover(pass, hovered)
    }

    #[inline(always)]
    fn on_focus(&mut self, pass: &mut EventPass<'_>, focused: bool) {
        self.on_focus(pass, focused)
    }

    #[inline(always)]
    fn on_child_focus(&mut self, pass: &mut EventPass<'_>, focused: bool) {
        self.on_child_focus(pass, focused)
    }
}

pub struct OnHover<E: Element> {
    pub element: E,
    pub callback: fn(&mut E, &mut EventPass<'_>, bool),
}

impl<E: Element> OnHover<E> {
    #[inline(always)]
    pub const fn new(element: E, callback: fn(&mut E, &mut EventPass<'_>, bool)) -> Self {
        Self { element, callback }
    }
}

impl<E: Element> ExtensionElement for OnHover<E> {
    #[inline(always)]
    fn element(&self) -> &dyn Element {
        &self.element
    }

    #[inline(always)]
    fn element_mut(&mut self) -> &mut dyn Element {
        &mut self.element
    }

    #[inline(always)]
    fn on_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {
        (self.callback)(&mut self.element, pass, hovered)
    }
}

pub struct OnClick<E: Element> {
    pub element: E,
    pub callback: fn(&mut E, &mut EventPass<'_>, bool),
}

impl<E: Element> OnClick<E> {
    #[inline(always)]
    pub const fn new(element: E, callback: fn(&mut E, &mut EventPass<'_>, bool)) -> Self {
        Self { element, callback }
    }
}

impl<E: Element> ExtensionElement for OnClick<E> {
    #[inline(always)]
    fn element(&self) -> &dyn Element {
        &self.element
    }

    #[inline(always)]
    fn element_mut(&mut self) -> &mut dyn Element {
        &mut self.element
    }

    #[inline(always)]
    fn on_pointer_event(&mut self, pass: &mut EventPass<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down {
                button: PointerButton::Primary,
                position: _,
            } => {
                pass.capture_pointer();
                (self.callback)(&mut self.element, pass, true)
            }
            PointerEvent::Up {
                button: PointerButton::Primary,
            } => (self.callback)(&mut self.element, pass, false),
            other => self.element.on_pointer_event(pass, other),
        }
    }
}

#[macro_export]
macro_rules! column {
    (@_ { $col:expr } gap: $gap:expr; $($rest:tt)*) => {
        $crate::column!(@_ { $col .with_gap($gap) } $($rest)*)
    };
    (@_ { $col:expr } $($rest:expr),* $(,)?) => {
        $col $(.with($rest))*
    };
    ($($items:tt)*) => {
        $crate::column!(@_ { Column::new() } $($items)*)
    };
}

#[macro_export]
macro_rules! row {
    (@_ { $row:expr } gap: $gap:expr; $($rest:tt)*) => {
        $crate::row!(@_ { $row .with_gap($gap) } $($rest)*)
    };
    (@_ { $row:expr } $($rest:expr),* $(,)?) => {
        $row $(.with($rest))*
    };
    ($($items:tt)*) => {
        $crate::row!(@_ { Row::new() } $($items)*)
    };
}

pub struct Column {
    children: Vec<ChildElement>,
    background_color: Rgba<u8>,
    border_color: Rgba<u8>,
    gap: f32,
}

impl Column {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            background_color: Rgba {
                r: 33,
                g: 33,
                b: 33,
                a: 255,
            },
            border_color: Rgba {
                r: 111,
                g: 111,
                b: 111,
                a: 255,
            },
            gap: 0.0,
        }
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
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

    fn render(&mut self, pass: &mut RenderPass<'_>) {
        pass.fill_quad(pass.bounds(), self.background_color, 1.0, self.border_color);
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

            y_offset += child_size.y + self.gap;
        }
    }

    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32 {
        let length_request = match length_request {
            LengthRequest::MinContent | LengthRequest::MaxContent => length_request,
            LengthRequest::FitContent(_space) => LengthRequest::MinContent,
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

        if axis == Axis::Vertical {
            let gap_count = (self.children.len() - 1) as f32;
            length += gap_count * self.gap;
        }

        length
    }

    fn on_child_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {
        if hovered {
            self.border_color = Rgba {
                r: 133,
                g: 133,
                b: 133,
                a: 255,
            };
        } else {
            self.border_color = Rgba {
                r: 111,
                g: 111,
                b: 111,
                a: 255,
            };
        }
        pass.request_render();
    }
}

pub struct Row {
    children: Vec<ChildElement>,
    background_color: Rgba<u8>,
    border_color: Rgba<u8>,
    gap: f32,
}

impl Row {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            background_color: Rgba {
                r: 33,
                g: 33,
                b: 33,
                a: 255,
            },
            border_color: Rgba {
                r: 111,
                g: 111,
                b: 111,
                a: 255,
            },
            gap: 0.0,
        }
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn with(mut self, child: impl Element + 'static) -> Self {
        self.children.push(ElementBuilder::new(child).into_child());
        self
    }
}

impl Element for Row {
    fn children_ids(&self) -> Vec<u64> {
        self.children.iter().map(|child| child.id()).collect()
    }

    fn update_children(&mut self, pass: &mut UpdatePass<'_>) {
        for child in self.children.iter_mut() {
            pass.update_child(child);
        }
    }

    fn render(&mut self, pass: &mut RenderPass<'_>) {
        pass.fill_quad(pass.bounds(), self.background_color, 1.0, self.border_color);
    }

    fn layout(&mut self, pass: &mut LayoutPass<'_>) {
        let width = Length::FitContent(pass.size.x);
        let height = Length::FitContent(pass.size.y);
        let auto_size = Xy::new(width, height);

        let mut x_offset = 0.0;
        for child in &mut self.children {
            let child_size = pass.resolve_size(child.id(), auto_size);
            pass.do_layout(child, child_size);
            pass.place_child(child, Xy::new(x_offset, 0.0));

            x_offset += child_size.x + self.gap;
        }
    }

    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32 {
        let length_request = match length_request {
            LengthRequest::MinContent | LengthRequest::MaxContent => length_request,
            LengthRequest::FitContent(_space) => LengthRequest::MinContent,
        };

        let fallback_length = length_request.into();

        let mut length: f32 = 0.0;
        for child in &mut self.children {
            let child_length =
                context.resolve_length(child.id(), axis, fallback_length, cross_length);
            match axis {
                Axis::Horizontal => length += child_length,
                Axis::Vertical => length = length.max(child_length),
            }
        }

        if axis == Axis::Horizontal {
            let gap_count = (self.children.len() - 1) as f32;
            length += gap_count * self.gap;
        }

        length
    }

    fn on_child_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {
        if hovered {
            self.border_color = Rgba {
                r: 133,
                g: 133,
                b: 133,
                a: 255,
            };
        } else {
            self.border_color = Rgba {
                r: 111,
                g: 111,
                b: 111,
                a: 255,
            };
        }
        pass.request_render();
    }
}

pub struct ScrollBar {
    progress: f32,
    area_height: f32,
    content_height: f32,
    grab_anchor: Option<f32>,
    moved: bool,
}

impl ScrollBar {
    pub fn new() -> Self {
        Self {
            progress: 0.0,
            area_height: 0.0,
            content_height: 0.0,
            grab_anchor: None,
            moved: false,
        }
    }
}

impl Element for ScrollBar {
    fn render(&mut self, pass: &mut RenderPass<'_>) {
        let height_ratio = if self.content_height != 0.0 {
            self.area_height / self.content_height
        } else {
            1.0
        };
        let height_ratio = height_ratio.clamp(0.0, 1.0);
        let min_height = 40.0; // TODO: Theme.
        let layout_size = pass.bounds().size();
        let bar_height = (height_ratio * layout_size.y).max(min_height);
        let empty_space = layout_size.y - bar_height;

        pass.fill_quad(
            Aabb2D::from_size_position(
                Xy::new(layout_size.x, bar_height),
                pass.bounds().position() + Xy::new(0.0, self.progress * empty_space),
            ),
            if self.grab_anchor.is_some() {
                Rgba {
                    r: 0x73,
                    g: 0x73,
                    b: 0x89,
                    a: 255,
                }
            } else {
                Rgba {
                    r: 0x53,
                    g: 0x53,
                    b: 0x6d,
                    a: 255,
                }
            },
            0.0,
            Rgba::NONE,
        );
    }

    fn layout(&mut self, _pass: &mut LayoutPass<'_>) {}

    fn measure(
        &mut self,
        _context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        _cross_length: Option<f32>,
    ) -> f32 {
        if axis == Axis::Vertical {
            match length_request {
                LengthRequest::MinContent | LengthRequest::MaxContent => self.area_height,
                LengthRequest::FitContent(space) => space,
            }
        } else {
            let scrollbar_width = 12.0; // TODO: Theming

            scrollbar_width
        }
    }

    fn cursor_icon(&self) -> CursorIcon {
        if self.grab_anchor.is_some() {
            CursorIcon::Grabbing
        } else {
            CursorIcon::Grab
        }
    }

    fn on_pointer_event(&mut self, pass: &mut EventPass<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down {
                position: mouse_pos,
                ..
            } => {
                pass.capture_pointer();

                let size = pass.bounds().size();
                let height_ratio = if self.content_height != 0.0 {
                    self.area_height / self.content_height
                } else {
                    1.0
                };
                let height_ratio = height_ratio.clamp(0.0, 1.0);
                let min_height = 40.0; // TODO: Theme.
                let bar_height = (height_ratio * size.y).max(min_height);
                let empty_space = size.y - bar_height;

                let bar_bounds = Aabb2D::from_size_position(
                    Xy::new(size.x, bar_height),
                    pass.bounds().position() + Xy::new(0.0, self.progress * empty_space),
                );

                // let mouse_pos = pass.local_position(*mouse_pos);
                let mut changed = false;
                if bar_bounds.contains(*mouse_pos) {
                    let y_min = bar_bounds.min.y;
                    let y_max = bar_bounds.max.y;
                    self.grab_anchor = Some((mouse_pos.y - y_min) / (y_max - y_min));
                } else {
                    let height_ratio = if self.content_height != 0.0 {
                        self.area_height / self.content_height
                    } else {
                        1.0
                    };
                    let height_ratio = height_ratio.clamp(0.0, 1.0);
                    let min_height = 40.0; // TODO: Theme.
                    let bar_height = (height_ratio * size.y).max(min_height);
                    let empty_space = size.y - bar_height;

                    let progress = (mouse_pos.y - bar_height * 0.5) / empty_space;
                    let progress = progress.clamp(0.0, 1.0);

                    changed |= (progress - self.progress).abs() > 1e-12;

                    self.progress = progress;
                    self.grab_anchor = Some(0.5);
                };
                if changed {
                    pass.request_render();
                }
            }
            PointerEvent::Move {
                position: mouse_pos,
            } => {
                if let Some(grab_anchor) = self.grab_anchor {
                    let size = pass.bounds().size();
                    let height_ratio = if self.content_height != 0.0 {
                        self.area_height / self.content_height
                    } else {
                        1.0
                    };
                    let height_ratio = height_ratio.clamp(0.0, 1.0);
                    let min_height = 40.0; // TODO: Theme.
                    let bar_height = (height_ratio * size.y).max(min_height);
                    let empty_space = size.y - bar_height;

                    let progress = (mouse_pos.y - bar_height * grab_anchor) / empty_space;
                    let progress = progress.clamp(0.0, 1.0);
                    if (progress - self.progress).abs() > 1e-12 {
                        self.progress = progress;
                        self.moved = true;
                        pass.request_render();
                    }
                }
            }
            PointerEvent::Up { .. } => {
                self.grab_anchor = None;
            }
            _ => {}
        }
    }
}

pub struct VerticalScroll {
    column: TypedChildElement<Column>,
    scroll_bar: TypedChildElement<ScrollBar>,
    viewport_offset: Xy<f32>,
    content_size: Xy<f32>,
}

impl VerticalScroll {
    pub fn new(column: Column) -> Self {
        Self {
            column: TypedChildElement::new(column),
            scroll_bar: TypedChildElement::new(ScrollBar::new()),
            viewport_offset: Xy::ZERO,
            content_size: Xy::ZERO,
        }
    }
}

impl Element for VerticalScroll {
    fn children_ids(&self) -> Vec<u64> {
        vec![self.column.id(), self.scroll_bar.id()]
    }

    fn update_children(&mut self, pass: &mut UpdatePass<'_>) {
        pass.update_child(&mut self.column.inner);
        pass.update_child(&mut self.scroll_bar.inner);
    }

    fn layout(&mut self, pass: &mut LayoutPass<'_>) {
        let auto_size = Xy::new(Length::FitContent(pass.size.x), Length::MaxContent);
        self.content_size = pass.resolve_size(self.column.id(), auto_size);

        pass.do_layout(&mut self.column.inner, self.content_size);
        pass.place_child(&mut self.column.inner, Xy::ZERO);

        let viewport_max_pos = (self.content_size - pass.size).max(Xy::ZERO);
        let pos = Xy::new(
            self.viewport_offset.x.clamp(0.0, viewport_max_pos.x),
            self.viewport_offset.y.clamp(0.0, viewport_max_pos.y),
        );
        if (pos - self.viewport_offset).length_squared() > 1e-12 {
            self.viewport_offset = pos;
        }

        {
            let area_size = pass.size;
            let scroll_bar = pass.typed_child_mut(&mut self.scroll_bar);
            scroll_bar.area_height = area_size.y;
            scroll_bar.content_height = self.content_size.y;
            pass.request_child_render(self.scroll_bar.id());
        }

        let scroll_bar_size = pass.resolve_size(
            self.scroll_bar.id(),
            Xy::new(
                Length::FitContent(pass.size.x),
                Length::FitContent(pass.size.y),
            ),
        );
        pass.do_layout(&mut self.scroll_bar.inner, scroll_bar_size);
        pass.place_child(
            &mut self.scroll_bar.inner,
            Xy::new(pass.size.x - scroll_bar_size.x, 0.0),
        );
    }

    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32 {
        match length_request {
            LengthRequest::MaxContent => {
                context.resolve_length(self.column.id(), axis, Length::MaxContent, cross_length)
            }
            LengthRequest::MinContent => 0.0,
            LengthRequest::FitContent(space) => space,
        }
    }

    fn compose(&mut self, pass: &mut ComposePass<'_>) {
        pass.set_child_scroll(
            &mut self.column.inner,
            Xy::new(-self.viewport_offset.x, -self.viewport_offset.y),
        );
    }

    fn on_pointer_event(&mut self, pass: &mut EventPass<'_>, event: &PointerEvent) {
        let scroll_range = (self.content_size - pass.state.bounds.size()).max(Xy::ZERO);

        let mut changed = false;
        match event {
            PointerEvent::Scroll { delta } => {
                let pixel_delta = delta.to_pixels(Xy::new(120.0, 120.0));
                let delta = Xy::new(0.0, pixel_delta.y);
                let pos = self.viewport_offset - delta;
                let pos = Xy::new(
                    pos.x.clamp(0.0, scroll_range.x),
                    pos.y.clamp(0.0, scroll_range.y),
                );

                if (pos - self.viewport_offset).length_squared() > 1e-12 {
                    changed = true;
                    self.viewport_offset = pos;
                    pass.set_handled();
                }
            }
            _ => {}
        }
        {
            let scroll_bar = pass.typed_child_mut(&mut self.scroll_bar);
            if scroll_bar.moved {
                scroll_bar.moved = false;
                let y = scroll_bar.progress * scroll_range.y;
                let pos = Xy::new(self.viewport_offset.x, y.clamp(0.0, scroll_range.y));
                if (pos - self.viewport_offset).length_squared() > 1e-12 {
                    changed = true;
                    self.viewport_offset = pos;
                }
            }
        }

        if changed {
            pass.set_handled();
            pass.request_compose();
            let progress = if scroll_range.y > 1e-12 {
                (self.viewport_offset.y / scroll_range.y).clamp(0.0, 1.0)
            } else {
                0.0
            };

            {
                let scroll_bar = pass.typed_child_mut(&mut self.scroll_bar);
                scroll_bar.progress = progress;
                pass.request_child_render(self.scroll_bar.id());
            }
        }
    }
}

pub struct Label {
    pub text: Arc<str>,
    pub font_size: f32,
    pub color: Rgba<u8>,
    // pub visual_font_size: AnimatedF32,
    pub line_height: LineHeight,
    pub font_style: FontStyle,
    pub alignment: TextAlignment,
    pub wrap_mode: TextWrapMode,
}

impl Label {
    pub fn new(text: impl Into<Arc<str>>) -> Self {
        Self {
            text: text.into(),
            color: Rgba::WHITE,
            font_size: 16.0,
            line_height: LineHeight::FONT_PREFERRED,
            font_style: FontStyle::Normal,
            alignment: TextAlignment::Start,
            wrap_mode: TextWrapMode::Wrap,
            // visual_font_size: AnimatedF32::new(16.0),
        }
    }

    pub fn with_color(mut self, color: Rgba<u8>) -> Self {
        self.color = color;
        self
    }

    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        // self.visual_font_size = AnimatedF32::new(font_size);
        self
    }
}

impl Element for Label {
    fn children_ids(&self) -> Vec<u64> {
        unsafe { __ui_Label__children_ids(self) }
    }

    fn render(&mut self, pass: &mut RenderPass<'_>) {
        unsafe { __ui_Label__render(self, pass) }
    }

    // fn animate(&mut self, pass: &mut AnimatePass<'_>, dt: f64) {
    //     let ms = (dt * 1000.0) as f32;
    //     let done = self.visual_font_size.advance(ms);
    //     if !done {
    //         pass.request_animate();
    //     }
    //     pass.request_render();
    // }

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

    fn cursor_icon(&self) -> CursorIcon {
        CursorIcon::IBeam
    }

    fn on_pointer_event(&mut self, pass: &mut EventPass<'_>, event: &PointerEvent) {
        if matches!(
            event,
            PointerEvent::Down {
                button: PointerButton::Primary,
                ..
            },
        ) {
            pass.request_focus();
        }
    }

    // fn on_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {
    //     if hovered {
    //         self.visual_font_size.move_to(self.font_size * 2.0, 1000.0);
    //     } else {
    //         self.visual_font_size.move_to(self.font_size, 1000.0);
    //     }
    //     pass.request_animate();
    //     pass.request_render();
    //     pass.set_handled();
    // }

    fn on_focus(&mut self, pass: &mut EventPass<'_>, focused: bool) {
        if focused {
            self.font_size *= 2.0;
        } else {
            self.font_size /= 2.0;
        }
        pass.request_layout();
        pass.request_render();
        pass.set_handled();
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

pub struct LineInput {
    pub text: String,
    pub font_size: f32,

    cursor_offset: usize,
    width_before_cursor: f32,
    width_after_cursor: f32,
    show_cursor: bool,
}

impl LineInput {
    pub fn new(text: impl ToString) -> Self {
        let text = text.to_string();
        let cursor_offset = text.len();
        Self {
            text,
            font_size: 16.0,
            cursor_offset,
            width_before_cursor: 0.0,
            width_after_cursor: 0.0,
            show_cursor: false,
        }
    }

    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }
}

impl Element for LineInput {
    fn render(&mut self, pass: &mut RenderPass<'_>) {
        pass.fill_quad(
            pass.bounds(),
            Rgba::NONE,
            1.0,
            Rgba {
                r: 111,
                g: 111,
                b: 111,
                a: 255,
            },
        );
        pass.fill_text(
            &self.text,
            pass.bounds()
                .with_width(self.width_before_cursor + self.width_after_cursor),
            Rgba {
                r: 177,
                g: 177,
                b: 177,
                a: 255,
            },
            self.font_size,
        );

        if self.show_cursor {
            let cursor_size = Xy::new(2.0, pass.bounds().size().y);
            let cursor_pos = pass.bounds().position() + Xy::new(self.width_before_cursor, 0.0);

            pass.fill_quad(
                Aabb2D::from_size_position(cursor_size, cursor_pos),
                Rgba {
                    r: 177,
                    g: 177,
                    b: 177,
                    a: 200,
                },
                0.0,
                Rgba::NONE,
            );
        }
    }

    fn layout(&mut self, _pass: &mut LayoutPass<'_>) {}

    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        _cross_length: Option<f32>,
    ) -> f32 {
        let id = context.id();
        let fonts = context.fonts_mut();
        let max_advance = match axis {
            Axis::Horizontal => match length_request {
                LengthRequest::MinContent | LengthRequest::MaxContent => None,
                LengthRequest::FitContent(space) => Some(space),
            },
            Axis::Vertical => None,
        };
        let before_cursor_size = fonts.measure_text(
            id,
            &self.text[..self.cursor_offset],
            max_advance,
            self.font_size,
            LineHeight::Relative(1.0),
            FontStyle::Normal,
            TextAlignment::Start,
            TextWrapMode::NoWrap,
        );
        let after_cursor_size = fonts.measure_text(
            id,
            &self.text[self.cursor_offset..],
            max_advance,
            self.font_size,
            LineHeight::Relative(1.0),
            FontStyle::Normal,
            TextAlignment::Start,
            TextWrapMode::NoWrap,
        );

        self.width_before_cursor = before_cursor_size.x;
        self.width_after_cursor = after_cursor_size.x;

        match axis {
            Axis::Horizontal => match length_request {
                LengthRequest::MinContent | LengthRequest::MaxContent => {
                    before_cursor_size.x + after_cursor_size.x
                }
                LengthRequest::FitContent(space) => space,
            },
            Axis::Vertical => before_cursor_size.y,
        }
    }

    fn cursor_icon(&self) -> CursorIcon {
        CursorIcon::IBeam
    }

    fn on_keyboard_event(&mut self, pass: &mut EventPass<'_>, event: &KeyboardEvent) {
        match event {
            KeyboardEvent::Down { key } => {
                if self.cursor_offset > self.text.len() {
                    self.cursor_offset = self.text.len();
                }
                match key {
                    Key::Char(ch) => {
                        self.text.insert(self.cursor_offset, *ch);
                        self.cursor_offset = self.cursor_offset.saturating_add(1);
                    }
                    Key::Backspace => {
                        if self.cursor_offset == 0 {
                            return;
                        }
                        _ = self.text.remove(self.cursor_offset.saturating_sub(1));
                        self.cursor_offset = self.cursor_offset.saturating_sub(1);
                    }
                    Key::Delete => {
                        if self.cursor_offset >= self.text.len() {
                            return;
                        }
                        _ = self.text.remove(self.cursor_offset);
                    }
                    Key::ArrowLeft => {
                        self.cursor_offset = self.cursor_offset.saturating_sub(1);
                    }
                    Key::ArrowRight => {
                        if self.cursor_offset >= self.text.len() {
                            return;
                        }
                        self.cursor_offset = self.cursor_offset.saturating_add(1);
                    }
                    _ => {
                        return;
                    }
                }
                pass.request_layout();
                pass.request_render();
                pass.set_handled();
            }
            KeyboardEvent::Up { key: _ } => {}
        }
    }

    fn on_pointer_event(&mut self, pass: &mut EventPass<'_>, event: &PointerEvent) {
        if matches!(
            event,
            PointerEvent::Down {
                button: PointerButton::Primary,
                ..
            },
        ) {
            pass.request_focus();
        }
    }

    fn on_focus(&mut self, pass: &mut EventPass<'_>, focused: bool) {
        self.show_cursor = focused;
        pass.request_render();
        pass.set_handled();
    }
}



#[derive(Clone, Debug)]
pub struct AnimatedF32 {
    current: f32,
    target: f32,
    rate: f32,
}

impl AnimatedF32 {
    pub const fn new(value: f32) -> Self {
        Self {
            current: value,
            target: value,
            rate: 0.0,
        }
    }

    #[inline]
    pub const fn get(&self) -> f32 {
        self.current
    }

    pub fn move_to(&mut self, target: f32, time_ms: f32) {
        self.target = target;
        match time_ms.partial_cmp(&0.0) {
            Some(std::cmp::Ordering::Equal | std::cmp::Ordering::Less) => self.current = target,
            Some(std::cmp::Ordering::Greater) => {
                self.rate = (self.target - self.current) / time_ms;
            }
            None => panic!(),
        }
    }

    pub fn advance(&mut self, ms: f32) -> bool {
        let original_cmp = self.current.partial_cmp(&self.target).unwrap();
        self.current += self.rate * ms;
        let final_cmp = self.current.partial_cmp(&self.target).unwrap();

        if final_cmp.is_eq() || original_cmp != final_cmp {
            self.current = self.target;
            self.rate = 0.0;
            true
        } else {
            false
        }
    }
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
    let node = view
        .tree
        .find_mut(view.root_element_id)
        .expect("failed to find the view's root node");

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

fn update_pointer_pass(view: &mut View) {
    let next_hovered_element = view
        .pointer_position
        .and_then(|pos| {
            find_pointer_target(
                view.tree
                    .find(view.root_element_id)
                    .expect("failed to find the view's root node"),
                pos,
            )
        })
        .map(|node| node.id());
    let next_hovered_path = next_hovered_element.map_or(Vec::new(), |node_id| {
        view.tree.branches().get_id_path(node_id, None)
    });
    let prev_hovered_path = std::mem::take(&mut view.hovered_path);
    let prev_hovered_element = prev_hovered_path.first().copied();

    if prev_hovered_path != next_hovered_path {
        let mut hovered_set = HashSet::new();
        for node_id in &next_hovered_path {
            hovered_set.insert(*node_id);
        }

        for node_id in prev_hovered_path.iter().copied() {
            if view
                .tree
                .find_mut(node_id)
                .map(|node| node.element.state.hovered != hovered_set.contains(&node_id))
                .unwrap_or(false)
            {
                let hovered = hovered_set.contains(&node_id);
                event_pass(view, Some(node_id), |element, pass| {
                    if pass.state.hovered != hovered {
                        element.on_child_hover(pass, hovered);
                    }
                    pass.state.hovered = hovered;
                });
            }
        }
        for node_id in next_hovered_path.iter().copied() {
            if view
                .tree
                .find_mut(node_id)
                .map(|node| node.element.state.hovered != hovered_set.contains(&node_id))
                .unwrap_or(false)
            {
                let hovered = hovered_set.contains(&node_id);
                event_pass(view, Some(node_id), |element, pass| {
                    if pass.state.hovered != hovered {
                        element.on_child_hover(pass, hovered);
                    }
                    pass.state.hovered = hovered;
                });
            }
        }
    }

    if prev_hovered_element != next_hovered_element {
        single_event_pass(view, prev_hovered_element, |element, pass| {
            pass.state.hovered = false;
            element.on_hover(pass, false);
        });
        single_event_pass(view, next_hovered_element, |element, pass| {
            pass.state.hovered = true;
            element.on_hover(pass, true);
        });
    }

    let next_cursor_icon =
        if let Some(node_id) = view.pointer_capture_target.or(next_hovered_element) {
            let node = view
                .tree
                .find_mut(node_id)
                .expect("failed to find the view's root node");

            node.element.element.cursor_icon()
        } else {
            CursorIcon::Default
        };

    view.cursor_icon = next_cursor_icon;
    view.hovered_path = next_hovered_path;
}

fn update_focus_pass(view: &mut View) {
    let next_focused_element = view.next_focused_element;
    let next_focused_path = next_focused_element.map_or(Vec::new(), |node_id| {
        view.tree.branches().get_id_path(node_id, None)
    });
    let prev_focused_path = std::mem::take(&mut view.focused_path);
    let prev_focused_element = prev_focused_path.first().copied();

    if prev_focused_path != next_focused_path {
        let mut focused_set = HashSet::new();
        for node_id in &next_focused_path {
            focused_set.insert(*node_id);
        }

        for node_id in prev_focused_path.iter().copied() {
            if view
                .tree
                .find_mut(node_id)
                .map(|node| node.element.state.focused != focused_set.contains(&node_id))
                .unwrap_or(false)
            {
                let focused = focused_set.contains(&node_id);
                event_pass(view, Some(node_id), |element, pass| {
                    if pass.state.focused != focused {
                        element.on_child_focus(pass, focused);
                    }
                    pass.state.focused = focused;
                });
            }
        }
        for node_id in next_focused_path.iter().copied() {
            if view
                .tree
                .find_mut(node_id)
                .map(|node| node.element.state.focused != focused_set.contains(&node_id))
                .unwrap_or(false)
            {
                let focused = focused_set.contains(&node_id);
                event_pass(view, Some(node_id), |element, pass| {
                    if pass.state.focused != focused {
                        element.on_child_focus(pass, focused);
                    }
                    pass.state.focused = focused;
                });
            }
        }
    }

    if prev_focused_element != next_focused_element {
        single_event_pass(view, prev_focused_element, |element, pass| {
            pass.state.focused = false;
            element.on_focus(pass, false);
        });
        single_event_pass(view, next_focused_element, |element, pass| {
            pass.state.focused = true;
            element.on_focus(pass, true);
        });
    }

    view.focused_element = next_focused_element;
    view.focused_path = next_focused_path;
}



pub struct EventPass<'view> {
    state: &'view mut ElementState,
    children: tree::LeavesMut<'view, ElementInfo>,
    handled: bool,
    next_focus: &'view mut Option<u64>,
    pointer_capture_target: &'view mut Option<u64>,
}

impl EventPass<'_> {
    pub fn set_handled(&mut self) {
        self.handled = true;
    }

    pub fn request_focus(&mut self) {
        *self.next_focus = Some(self.state.id);
    }

    pub fn capture_pointer(&mut self) {
        *self.pointer_capture_target = Some(self.state.id);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)] // TODO: Finish adding the extra pointer buttons (up to button 32).
pub enum PointerButton {
    // TODO: Nullable type?
    Primary = 1,
    Secondary = 1 << 1,
    Auxiliary = 1 << 2,
    Back = 1 << 3,
    Forward = 1 << 4,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub enum PointerEvent {
    Down {
        button: PointerButton,
        position: Xy<f32>,
    },
    Up {
        button: PointerButton,
    },
    Move {
        position: Xy<f32>,
    },
    Scroll {
        delta: ScrollDelta,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollDelta {
    Pixels(Xy<f32>),
    Lines(Xy<f32>),
}

impl ScrollDelta {
    pub fn to_pixels(self, line_size: Xy<f32>) -> Xy<f32> {
        match self {
            Self::Pixels(delta) => delta,
            ScrollDelta::Lines(delta) => delta * line_size,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(C)]
pub enum Key {
    Char(char),

    Space,
    Tab,
    Enter,
    Backspace,
    Delete,

    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    PageUp,
    PageDown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub enum KeyboardEvent {
    Down { key: Key },
    Up { key: Key },
}

fn event_pass(
    view: &mut View,
    target: Option<u64>,
    mut callback: impl FnMut(&mut dyn Element, &mut EventPass<'_>),
) {
    let mut target_id = target;
    let mut handled = false;
    while let Some(node_id) = target_id {
        let parent_id = {
            let node = view
                .tree
                .find_mut(node_id)
                .expect("invalid element ID for event target");

            if !handled {
                let mut pass = EventPass {
                    state: &mut node.element.state,
                    children: node.leaves,
                    handled: false,
                    next_focus: &mut view.next_focused_element,
                    pointer_capture_target: &mut view.pointer_capture_target,
                };
                callback(&mut *node.element.element, &mut pass);

                handled = pass.handled;
            }

            node.branch_id
        };

        if let Some(parent_id) = parent_id {
            let mut parent_node = view.tree.find_mut(parent_id).unwrap();
            let node = parent_node.leaves.get_mut(node_id).unwrap();

            parent_node
                .element
                .state
                .merge_with_child(&node.element.state);
        }

        target_id = parent_id;
    }
}

fn single_event_pass(
    view: &mut View,
    target: Option<u64>,
    mut callback: impl FnMut(&mut dyn Element, &mut EventPass<'_>),
) {
    let Some(target) = target else {
        return;
    };

    let node = view
        .tree
        .find_mut(target)
        .expect("invalid element ID passed to single_event_pass");

    let mut pass = EventPass {
        state: &mut node.element.state,
        children: node.leaves,
        handled: false,
        next_focus: &mut view.next_focused_element,
        pointer_capture_target: &mut view.pointer_capture_target,
    };
    callback(&mut *node.element.element, &mut pass);

    let mut current_id = Some(target);
    while let Some(node_id) = current_id {
        let parent_id = view
            .tree
            .find_mut(node_id)
            .expect("invalid element ID for pointer target")
            .branch_id;
        if let Some(parent_id) = parent_id {
            let mut parent_node = view.tree.find_mut(parent_id).unwrap();
            let node = parent_node.leaves.get_mut(node_id).unwrap();

            parent_node
                .element
                .state
                .merge_with_child(&node.element.state);
        }

        current_id = parent_id;
    }
}

fn keyboard_event_pass(view: &mut View, event: &KeyboardEvent) {
    event_pass(view, view.focused_element, |element, pass| {
        element.on_keyboard_event(pass, event)
    });
}

fn pointer_event_pass(view: &mut View, event: &PointerEvent) {
    // let mut pointer_entered = false;
    if let PointerEvent::Move { position } = &event {
        if view.pointer_position == Some(*position) {
            return;
        }
        // pointer_entered = view.pointer_position.is_none();
        view.pointer_position = Some(*position);
    }
    let pointer_target = get_pointer_target(&view, view.pointer_position);

    if matches!(event, PointerEvent::Down { .. })
        && let Some(target_id) = pointer_target
    {
        // Clear the focus when the user clicks outside the focused element.
        if let Some(focused_element) = view.focused_element {
            // Focused element isn't an ancestor of the pointer target.
            if !view
                .tree
                .branches()
                .get_id_path(target_id, None)
                .contains(&focused_element)
            {
                view.next_focused_element = None;
            }
        }
    }

    event_pass(view, pointer_target, |element, pass| {
        element.on_pointer_event(pass, event)
    });

    if matches!(event, PointerEvent::Up { .. }) {
        view.pointer_capture_target = None;
    }
}

fn get_pointer_target(view: &View, pointer_pos: Option<Xy<f32>>) -> Option<u64> {
    if let Some(capture_target) = view.pointer_capture_target
        && view.tree.find(capture_target).is_some()
    {
        return Some(capture_target);
    }

    if let Some(pointer_pos) = pointer_pos {
        return find_pointer_target(
            view.tree
                .find(view.root_element_id)
                .expect("failed to find the view's root node"),
            pointer_pos,
        )
        .map(|node| node.id());
    }

    None
}

fn find_pointer_target<'view>(
    node: tree::NodeRef<'view, ElementInfo>,
    position: Xy<f32>,
) -> Option<tree::NodeRef<'view, ElementInfo>> {
    if !node.element.state.bounds.contains(position) {
        return None;
    }

    for child_id in node.element.element.children_ids().iter().rev() {
        if let Some(child) = find_pointer_target(
            node.leaves
                .reborrow_up()
                .get_into(*child_id)
                .expect("passed invalid child ID to find_pointer_target"),
            position,
        ) {
            return Some(child);
        }
    }

    if node.element.element.accepts_pointer_events() {
        // && ctx.size().to_rect().contains(local_pos) {
        Some(node)
    } else {
        None
    }
}



#[derive(Clone, Debug)]
#[repr(C)]
pub struct SizedVec<T: Sized, const SIZE: usize> {
    inner: [Option<T>; SIZE],
}

impl<T: Sized + Clone + Debug, const SIZE: usize> Default for SizedVec<T, SIZE> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T: Sized + Clone + Debug, const SIZE: usize> SizedVec<T, SIZE> {
    pub fn empty() -> Self {
        const {
            assert!(SIZE > 0);
        }

        Self {
            inner: (0..SIZE)
                .map(|_| None)
                .collect::<Vec<_>>()
                .try_into()
                .expect("length should equal SIZE"),
        }
    }

    pub fn push(&mut self, element: T) -> Option<T> {
        if let Some(null_index) = self.inner.iter().position(|item| item.is_none()) {
            self.inner[null_index] = Some(element);
            None
        } else {
            Some(element)
        }
    }

    pub fn clear(&mut self) {
        self.inner.iter_mut().for_each(|item| {
            item.take();
        });
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner.iter().flat_map(|item| item.as_ref())
    }
}



#[derive(Default)]
#[repr(C)]
pub struct Render {
    pub commands: SizedVec<RenderCommand, 512>,
}

#[derive(Clone, Debug)]
pub struct RenderQuad {
    pub bounds: Aabb2D<f32>,
    pub color: Rgba<u8>,
    pub border_width: f32,
    pub border_color: Rgba<u8>,
}

#[derive(Clone, Debug)]
pub struct RenderText {
    pub content: Arc<str>,
    pub bounds: Aabb2D<f32>,
    pub color: Rgba<u8>,
    pub font_size: f32,
}

impl Render {
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    fn extend(&mut self, other: &CachedRender) {
        for command in other.commands.iter().cloned() {
            self.commands.push(command);
        }
    }
}

#[derive(Default)]
struct CachedRender {
    commands: Vec<RenderCommand>,
}

impl CachedRender {
    fn clear(&mut self) {
        self.commands.clear();
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub enum RenderCommand {
    DrawChar(char),
    DrawQuad,
    SetBounds(Aabb2D<f32>),
    SetForegroundColor(Rgba<u8>),
    SetBackgroundColor(Rgba<u8>),
    SetBorderColor(Rgba<u8>),
    SetBorderWidth(f32),
    SetFontSize(f32),
}

struct RenderPassVariables {
    bounds: Aabb2D<f32>,
    font_size: f32,
    foreground_color: Rgba<u8>,
    background_color: Rgba<u8>,
    border_color: Rgba<u8>,
    border_width: f32,
}

impl Default for RenderPassVariables {
    fn default() -> Self {
        Self {
            bounds: Aabb2D::ZERO,
            font_size: 16.0,
            foreground_color: Rgba::WHITE,
            background_color: Rgba::BLACK,
            border_color: Rgba::NONE,
            border_width: 0.0,
        }
    }
}

pub struct RenderPass<'view> {
    state: &'view mut ElementState,
    render: &'view mut CachedRender,
    vars: &'view mut RenderPassVariables,
}

impl<'view> RenderPass<'view> {
    fn new(
        state: &'view mut ElementState,
        render: &'view mut CachedRender,
        vars: &'view mut RenderPassVariables,
    ) -> Self {
        Self {
            state,
            render,
            vars,
        }
    }

    pub fn fill_quad(
        &mut self,
        bounds: Aabb2D<f32>,
        color: Rgba<u8>,
        border_width: f32,
        border_color: Rgba<u8>,
    ) {
        if bounds != self.vars.bounds {
            self.render.commands.push(RenderCommand::SetBounds(bounds));
            self.vars.bounds = bounds;
        }
        if color != self.vars.background_color {
            self.render
                .commands
                .push(RenderCommand::SetBackgroundColor(color));
            self.vars.background_color = color;
        }
        if border_width != self.vars.border_width {
            self.render
                .commands
                .push(RenderCommand::SetBorderWidth(border_width));
            self.vars.border_width = border_width;
        }
        if border_color != self.vars.border_color {
            self.render
                .commands
                .push(RenderCommand::SetBorderColor(border_color));
            self.vars.border_color = border_color;
        }

        self.render.commands.push(RenderCommand::DrawQuad);
    }

    pub fn fill_text(
        &mut self,
        content: impl AsRef<str>,
        bounds: Aabb2D<f32>,
        color: Rgba<u8>,
        font_size: f32,
    ) {
        if bounds != self.vars.bounds {
            self.render.commands.push(RenderCommand::SetBounds(bounds));
            self.vars.bounds = bounds;
        }
        if color != self.vars.foreground_color {
            self.render
                .commands
                .push(RenderCommand::SetForegroundColor(color));
            self.vars.foreground_color = color;
        }
        if font_size != self.vars.font_size {
            self.render
                .commands
                .push(RenderCommand::SetFontSize(font_size));
            self.vars.font_size = font_size;
        }

        for ch in content.as_ref().chars() {
            self.render.commands.push(RenderCommand::DrawChar(ch));
        }
    }
}

pub fn render_pass(view: &mut View, render: &mut Render) {
    render.clear();
    let root_node = view
        .tree
        .find_mut(view.root_element_id)
        .expect("failed to find the view's root node");
    let mut vars = RenderPassVariables::default();

    render_element(root_node, &mut view.render_cache, render, &mut vars);
}

fn render_element(
    node: tree::NodeMut<'_, ElementInfo>,
    render_cache: &mut HashMap<u64, (CachedRender, CachedRender)>,
    final_render: &mut Render,
    vars: &mut RenderPassVariables,
) {
    let children = node.leaves;
    let element = &mut *node.element.element;
    let state = &mut node.element.state;

    if state.wants_render || state.wants_overlay_render {
        let (render, overlay_render) = render_cache.entry(state.id).or_default();

        if state.wants_render {
            render.clear();
            let mut pass = RenderPass::new(state, render, vars);
            element.render(&mut pass);
        }
        if state.wants_overlay_render {
            overlay_render.clear();
            let mut pass = RenderPass::new(state, overlay_render, vars);
            element.render_overlay(&mut pass);
        }
    }

    state.needs_render = false;
    state.wants_render = false;
    state.wants_overlay_render = false;

    {
        let Some((render, _)) = &mut render_cache.get(&state.id) else {
            return;
        };

        final_render.extend(render);
    }

    let parent_state = &mut *state;
    for_each_child_element(element, children, |mut node| {
        render_element(node.reborrow_mut(), render_cache, final_render, vars);
        parent_state.merge_with_child(&node.element.state);
    });

    {
        let Some((_, overlay_render)) = &mut render_cache.get(&state.id) else {
            return;
        };

        final_render.extend(overlay_render);
    }
}



pub struct AnimatePass<'view> {
    state: &'view mut ElementState,
    children: tree::LeavesMut<'view, ElementInfo>,
}

fn animation_pass(view: &mut View, time_delta: f64) {
    let node = view
        .tree
        .find_mut(view.root_element_id)
        .expect("failed to find the view's root node");
    animate_element(node, time_delta);
}

fn animate_element(node: tree::NodeMut<'_, ElementInfo>, time_delta: f64) {
    let mut children = node.leaves;
    let element = &mut *node.element.element;
    let state = &mut node.element.state;

    if !state.needs_animate {
        return;
    }
    state.needs_animate = false;

    if state.wants_animate {
        state.wants_animate = false;
        element.animate(
            &mut AnimatePass {
                state,
                children: children.reborrow_mut(),
            },
            time_delta,
        );
    }

    state.needs_render = true;

    let parent_state = &mut *state;
    for_each_child_element(element, children, |mut node| {
        animate_element(node.reborrow_mut(), time_delta);
        parent_state.merge_with_child(&node.element.state);
    });
}



pub struct ComposePass<'view> {
    state: &'view mut ElementState,
    children: tree::LeavesMut<'view, ElementInfo>,
}

impl ComposePass<'_> {
    pub fn set_child_scroll(&mut self, child: &mut ChildElement, translation: Xy<f32>) {
        let translation = translation.round();

        let child_state = &mut self
            .children
            .get_mut(child.id())
            .expect("invalid child passed to ComposePass::set_child_scroll_translation")
            .element
            .state;
        if translation != child_state.scroll_translation {
            child_state.scroll_translation = translation;
            child_state.transformed = true;
        }
    }
}

pub fn compose_pass(view: &mut View) {
    let node = view
        .tree
        .find_mut(view.root_element_id)
        .expect("failed to find the view's root node");
    compose_element(node, Transform2D::IDENTITY, false);
}

fn compose_element(
    node: tree::NodeMut<'_, ElementInfo>,
    parent_global_transform: Transform2D,
    parent_transformed: bool,
) {
    let mut children = node.leaves;
    let element = &mut *node.element.element;
    let state = &mut node.element.state;

    let transformed = parent_transformed || state.transformed;

    if !transformed && !state.needs_compose {
        return;
    }

    let local_translation = state.scroll_translation + state.layout_bounds.position();
    state.global_transform =
        parent_global_transform * state.local_transform.with_translation(local_translation);
    state.bounds = state
        .global_transform
        .transform_area(Aabb2D::from_size(state.layout_bounds.size()));

    if state.wants_compose {
        element.compose(&mut ComposePass {
            state,
            children: children.reborrow_mut(),
        });
    }

    state.needs_render = true;
    state.needs_compose = false;
    state.wants_compose = false;
    state.transformed = false;

    let parent_state = &mut *state;
    for_each_child_element(element, children, |mut node| {
        compose_element(
            node.reborrow_mut(),
            parent_state.global_transform,
            transformed,
        );
        parent_state.merge_with_child(&node.element.state);
    });
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
        .find_mut(view.root_element_id)
        .expect("failed to find the view's root node");
    layout_element(&mut *view.fonts, node, view.window_size);
}

fn layout_element(fonts: &mut dyn Fonts, node: tree::NodeMut<'_, ElementInfo>, size: Xy<f32>) {
    let element = &mut *node.element.element;
    let state = &mut node.element.state;
    let children = node.leaves;

    state.layout_bounds.set_size(size);

    let mut pass = LayoutPass {
        fonts,
        state,
        children,
        size,
    };
    element.layout(&mut pass);

    state.needs_render = true;
    state.wants_render = true;
    state.needs_compose = true;
    state.wants_compose = true;
}

fn move_element(state: &mut ElementState, position: Xy<f32>) {
    let end_point = position + state.layout_bounds.size();

    let position = position.round();
    let end_point = end_point.round();
    let baseline_offset = (end_point.y - state.layout_baseline_offset).round();

    if position != state.layout_bounds.min {
        state.transformed = true;
    }

    state.layout_bounds.min = position;
    state.layout_bounds.max = end_point;
    state.baseline_offset = baseline_offset;
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

multi_impl! {
    LayoutPass<'_>,
    MeasureContext<'_>,
    {
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
}

// Types with a `state: &mut ElementState` field.
multi_impl! {
    AnimatePass<'_>,
    ComposePass<'_>,
    EventPass<'_>,
    LayoutPass<'_>,
    MeasureContext<'_>,
    RenderPass<'_>,
    UpdatePass<'_>,
    {
        #[inline]
        pub fn id(&self) -> u64 {
            self.state.id
        }

        #[inline]
        pub fn bounds(&self) -> Aabb2D<f32> {
            self.state.bounds
        }

        pub fn local_position(&self, point: Xy<f32>) -> Xy<f32> {
            self.state.global_transform.inverse() * point
        }

        pub fn baseline_offset(&self) -> f32 {
            self.state.layout_bounds.max.y - self.state.baseline_offset
        }

        pub fn set_baseline_offset(&mut self, offset: f32) {
            self.state.layout_baseline_offset = offset;
        }

        pub fn clear_baseline_offset(&mut self) {
            self.state.layout_baseline_offset = 0.0;
        }

        pub fn request_render(&mut self) {
            self.state.wants_render = true;
        }

        pub fn request_layout(&mut self) {
            self.state.needs_layout = true;
        }

        pub fn request_compose(&mut self) {
            self.state.wants_compose = true;
            self.state.needs_compose = true;
        }

        pub fn request_animate(&mut self) {
            self.state.wants_animate = true;
            self.state.needs_animate = true;
        }
    }
}

// Types with a `children: tree::LeavesMut<'_, ElementInfo>` field.
multi_impl! {
    AnimatePass<'_>,
    ComposePass<'_>,
    EventPass<'_>,
    LayoutPass<'_>,
    MeasureContext<'_>,
    UpdatePass<'_>,
    {
        pub fn child(&self, id: u64) -> Option<tree::NodeRef<'_, ElementInfo>> {
            self.children.get(id)
        }

        pub fn expect_child(&self, id: u64) -> tree::NodeRef<'_, ElementInfo> {
            self.children.get(id).expect("invalid ID passed to `expect_child`")
        }

        pub fn typed_child_mut<T: Element>(
            &mut self,
            child: &mut TypedChildElement<T>,
        ) -> &mut T {
            let node_mut = self
                .children
                .get_mut(child.id())
                .expect("get_mut: child element not found");

            (&mut *node_mut.element.element as &mut dyn Any).downcast_mut().unwrap()
        }

        pub fn request_child_render(&mut self, id: u64) {
            self.children
                .get_mut(id)
                .expect("invalid child ID passed to request_child_render")
                .element
                .state
                .wants_render = true;
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
