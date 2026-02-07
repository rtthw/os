//! # Application Binary Interface (ABI)

pub mod cursor_icon;
pub mod elf;
pub mod layout;
pub mod math;
pub mod path;
pub mod stable_string;
pub mod stable_vec;
pub mod tree;
pub mod type_map;

pub use {
    cursor_icon::CursorIcon,
    math::{Aabb2D, Axis, Transform2D, Xy},
    path::Path,
    stable_string::StableString,
    stable_vec::StableVec,
    type_map::{TypeMap, TypeMapEntry},
};

use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
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
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 0xff }
    }
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
    root_element_id: u64,
    window_size: Xy<f32>,
    render_cache: HashMap<u64, (CachedRender, CachedRender)>,
    pointer_position: Option<Xy<f32>>,
    pointer_capture_target: Option<u64>,
    hovered_path: Vec<u64>,
    cursor_icon: CursorIcon,
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

        Self {
            fonts,
            tree,
            root_element_id: id,
            window_size,
            render_cache: HashMap::new(),
            pointer_position: None,
            pointer_capture_target: None,
            hovered_path: Vec::new(),
            cursor_icon: CursorIcon::Default,
        }
    }

    #[inline]
    pub fn cursor_icon(&self) -> CursorIcon {
        self.cursor_icon
    }

    pub fn resize_window(&mut self, size: Xy<f32>) {
        if self.window_size == size {
            return;
        }
        self.window_size = size;

        layout_pass(self);
    }

    pub fn handle_pointer_event(&mut self, event: PointerEvent) {
        pointer_event_pass(self, event);
        update_pointer_pass(self);
        layout_pass(self);
    }
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

    fn layout(&mut self, pass: &mut LayoutPass<'_>);

    fn measure(
        &mut self,
        context: &mut MeasureContext<'_>,
        axis: Axis,
        length_request: LengthRequest,
        cross_length: Option<f32>,
    ) -> f32;

    fn cursor_icon(&self) -> CursorIcon {
        CursorIcon::Default
    }

    /// Called when this element is added to the view tree.
    #[allow(unused)]
    fn on_build(&mut self, pass: &mut UpdatePass<'_>) {}

    /// Called when this element is interacted with by the user's pointer.
    #[allow(unused)]
    fn on_pointer_event(&mut self, pass: &mut EventPass<'_>) {}

    #[allow(unused)]
    fn on_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {}

    #[allow(unused)]
    fn on_child_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {}
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

    pub hovered: bool,
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
            hovered: false,
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



pub struct Column {
    children: Vec<ChildElement>,
    background_color: Rgba<u8>,
    border_color: Rgba<u8>,
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
        }
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

            x_offset += child_size.x;
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

    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
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

    fn on_hover(&mut self, pass: &mut EventPass<'_>, hovered: bool) {
        if hovered {
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

    let next_cursor_icon = if let Some(node_id) = next_hovered_element {
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



pub struct EventPass<'view> {
    state: &'view mut ElementState,
    children: tree::LeavesMut<'view, ElementInfo>,
    handled: bool,
}

impl EventPass<'_> {
    pub fn set_handled(&mut self) {
        self.handled = true;
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
    Down { button: PointerButton },
    Up { button: PointerButton },
    Move { position: Xy<f32> },
    Scroll { delta: ScrollDelta },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollDelta {
    Pixels(Xy<f32>),
    Lines(Xy<f32>),
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

fn pointer_event_pass(view: &mut View, event: PointerEvent) {
    // let mut pointer_entered = false;
    if let PointerEvent::Move { position } = &event {
        if view.pointer_position == Some(*position) {
            return;
        }
        // pointer_entered = view.pointer_position.is_none();
        view.pointer_position = Some(*position);
    }
    let pointer_target = get_pointer_target(&view, view.pointer_position);

    event_pass(view, pointer_target, |element, pass| {
        element.on_pointer_event(pass)
    });
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

    pub fn bounds(&self) -> Aabb2D<f32> {
        self.state.bounds
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
        let offset = self.state.bounds.position();
        move_element(
            &mut self
                .children
                .get_mut(child.id)
                .expect("invalid child passed to LayoutPass::place_child")
                .element
                .state,
            position + offset,
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

    state.bounds.set_size(size);

    let mut pass = LayoutPass {
        fonts,
        state,
        children,
        size,
    };
    element.layout(&mut pass);

    state.needs_render = true;
    state.wants_render = true;
}

fn move_element(state: &mut ElementState, position: Xy<f32>) {
    let end_point = position + state.bounds.size();

    let position = position.round();
    let end_point = end_point.round();

    if position != state.bounds.min {
        state.moved = true;
    }

    state.bounds.min = position;
    state.bounds.max = end_point;
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
    EventPass<'_>,
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
            self.state.needs_layout = true;
        }
    }
}

// Types with a `children: tree::LeavesMut<'_, ElementInfo>` field.
multi_impl! {
    EventPass<'_>,
    LayoutPass<'_>,
    MeasureContext<'_>,
    UpdatePass<'_>,
    {
        pub fn child(&self, id: u64) -> Option<tree::NodeRef<'_, ElementInfo>> {
            self.children.get(id)
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
