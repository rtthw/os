//! # Application Binary Interface (ABI)

#![no_std]

#[cfg(feature = "alloc")]
pub extern crate alloc;

pub mod elf;
pub mod layout;
pub mod path;
pub mod string;
pub mod vec;

pub use {path::Path, string::String, vec::Vec};

use {
    alloc::{boxed::Box, collections::vec_deque::VecDeque},
    core::{
        any::Any,
        ops::{Deref, DerefMut, Sub},
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
    pub const fn contains(&self, point: Xy<f32>) -> bool {
        point.x >= self.x_min
            && point.x <= self.x_max
            && point.y >= self.y_min
            && point.y <= self.y_max
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Xy<V> {
    pub x: V,
    pub y: V,
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



pub struct View {
    settings: ViewSettings,

    root: ViewNode,
    elements: slotmap::SlotMap<ViewNode, ElementInfo>,
    children: slotmap::SecondaryMap<ViewNode, alloc::vec::Vec<ViewNode>>,
    parents: slotmap::SecondaryMap<ViewNode, Option<ViewNode>>,

    events: VecDeque<ViewEvent>,

    mouse_pos: Xy<f32>,
    mouse_over: alloc::vec::Vec<ViewNode>,
    last_left_click_time: u64,
    last_left_click_node: Option<ViewNode>,
    // key_modifiers: ModifierKeyMask,
    focus: Option<ViewNode>,
}

pub struct ViewSettings {
    pub double_click_timeout: u64,
}

impl View {
    /// Apply the provided function to each element node in descending (back to
    /// front) order.
    pub fn for_each(&self, mut func: impl FnMut(&View, ViewNode)) {
        fn inner(view: &View, node: ViewNode, func: &mut impl FnMut(&View, ViewNode)) {
            func(view, node);
            for child in view.children[node].clone() {
                inner(view, child, func);
            }
        }

        inner(self, self.root, &mut func);
    }

    /// Apply the provided function to each element node in descending (back to
    /// front) order.
    pub fn for_each_mut(&mut self, mut func: impl FnMut(&mut View, ViewNode)) {
        fn inner(view: &mut View, node: ViewNode, func: &mut impl FnMut(&mut View, ViewNode)) {
            func(view, node);
            for child in view.children[node].clone() {
                inner(view, child, func);
            }
        }

        inner(self, self.root, &mut func);
    }


    pub fn handle_mouse_move(&mut self, position: Xy<f32>) {
        if self.mouse_pos == position {
            return;
        }
        self.mouse_pos = position;

        let delta = position - self.mouse_pos;
        // TODO: Setting for not reporting mouse movements to avoid clogging event
        // queue?
        self.events.push_back(ViewEvent::MouseMove { delta });

        fn inner(
            node: ViewNode,
            position: Xy<f32>,
            mouse_over: &mut alloc::vec::Vec<ViewNode>,
            elements: &mut slotmap::SlotMap<ViewNode, ElementInfo>,
            children: &mut slotmap::SecondaryMap<ViewNode, alloc::vec::Vec<ViewNode>>,
        ) {
            if !elements[node].bounds.contains(position) {
                return;
            }
            mouse_over.push(node);
            for child in children[node].clone() {
                inner(child, position, mouse_over, elements, children);
            }
        }

        let mut new_mouse_over = alloc::vec::Vec::new();
        inner(
            self.root,
            position,
            &mut new_mouse_over,
            &mut self.elements,
            &mut self.children,
        );

        if self.mouse_over != new_mouse_over {
            for node in &self.mouse_over {
                if !new_mouse_over.contains(node) {
                    self.events.push_back(ViewEvent::MouseLeave { node: *node });
                }
            }
            for node in &new_mouse_over {
                if !self.mouse_over.contains(node) {
                    self.events.push_back(ViewEvent::MouseEnter { node: *node });
                }
            }
            self.mouse_over = new_mouse_over;
        }
    }

    pub fn handle_mouse_down(&mut self, button: MouseButton, now: u64) {
        if self.mouse_over.is_empty() {
            return;
        }

        if let Some(node) = self
            .mouse_over
            .iter()
            .rev()
            .find(|node| self.elements[**node].clickable())
        {
            match button {
                MouseButton::Primary => {
                    self.events.push_back(ViewEvent::Click { node: *node });
                    if self
                        .last_left_click_node
                        .as_ref()
                        .is_some_and(|n| n == node)
                    {
                        if now.saturating_sub(self.last_left_click_time)
                            < self.settings.double_click_timeout
                        {
                            self.events
                                .push_back(ViewEvent::DoubleClick { node: *node });
                        }
                    } else {
                        self.last_left_click_node = Some(*node);
                    }
                    self.last_left_click_time = now;
                }
                MouseButton::Secondary => {
                    self.events.push_back(ViewEvent::RightClick { node: *node })
                }
                _ => {}
            }
        }
        if let Some(node) = self
            .mouse_over
            .iter()
            .rev()
            .find(|node| self.elements[**node].focusable())
        {
            if self.focus.as_ref().is_some_and(|n| n == node) {
                return;
            }
            match button {
                MouseButton::Primary | MouseButton::Secondary => {
                    let old = self.focus.take();
                    self.focus = Some(*node);
                    self.events.push_back(ViewEvent::Focus { old, new: *node });
                }
                _ => {}
            }
        }
    }
}

slotmap::new_key_type! { pub struct ViewNode; }

pub enum ViewEvent {
    MouseMove {
        delta: Xy<f32>,
    },
    MouseEnter {
        node: ViewNode,
    },
    MouseLeave {
        node: ViewNode,
    },
    Click {
        node: ViewNode,
    },
    RightClick {
        node: ViewNode,
    },
    DoubleClick {
        node: ViewNode,
    },
    Focus {
        old: Option<ViewNode>,
        new: ViewNode,
    },
}

pub trait Element {
    fn clickable(&self) -> bool {
        false
    }

    fn focusable(&self) -> bool {
        false
    }
}

pub struct ElementInfo {
    pub element: Box<dyn Element>,
    pub bounds: Aabb2D<f32>,
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
