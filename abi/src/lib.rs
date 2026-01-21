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
    alloc::{borrow::Cow, boxed::Box},
    core::any::Any,
};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");



pub trait App<U: Any> {
    fn view(&mut self, bounds: Aabb2D<f32>) -> impl View<U>;

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
            fn render(&mut self, renderer: &mut dyn Renderer) {
                self.app.view(renderer.bounds()).render(renderer);
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

            fn handle_event(
                &mut self,
                event: &InputEvent,
                bounds: Aabb2D<f32>,
                mouse_pos: Xy<f32>,
            ) -> Result<(), &'static str> {
                let mut updates = alloc::vec![];
                let mut captured = false;

                self.app.view(bounds).handle_input(
                    &mut updates,
                    event,
                    &mut captured,
                    bounds,
                    mouse_pos,
                );

                for update in updates {
                    self.app.update(update)?;
                }

                Ok(())
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
    fn render(&mut self, renderer: &mut dyn Renderer);
    /// Pass a type-erased update to the underlying [`App`].
    fn update(&mut self, update: Box<dyn Any>) -> Result<(), &'static str>;
    fn handle_event(
        &mut self,
        event: &InputEvent,
        bounds: Aabb2D<f32>,
        mouse_pos: Xy<f32>,
    ) -> Result<(), &'static str>;
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

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Xy<V> {
    pub x: V,
    pub y: V,
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



pub trait Renderer {
    fn bounds(&self) -> Aabb2D<f32>;
    fn label(&mut self, label: &Label<'_>);
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



pub struct ViewObject<'a, U> {
    inner: Box<dyn View<U> + 'a>,
}

impl<'a, U> ViewObject<'a, U> {
    pub fn new(view: impl View<U> + 'a) -> Self {
        Self {
            inner: Box::new(view),
        }
    }

    pub fn as_view(&self) -> &dyn View<U> {
        self.inner.as_ref()
    }

    pub fn as_view_mut(&mut self) -> &mut dyn View<U> {
        self.inner.as_mut()
    }
}

pub trait View<U> {
    #[allow(unused)]
    fn handle_input(
        &mut self,
        updates: &mut alloc::vec::Vec<U>,
        event: &InputEvent,
        captured: &mut bool,
        bounds: Aabb2D<f32>,
        mouse_pos: Xy<f32>,
    ) {
    }

    fn render(&self, renderer: &mut dyn Renderer);
}

pub struct Label<'a> {
    pub content: Cow<'a, str>,
    pub color: Rgba<u8>,
    pub font_size: f32,
}

impl<'a> Label<'a> {
    pub fn new(content: impl Into<Cow<'a, str>>) -> Self {
        Self {
            content: content.into(),
            color: Rgba {
                r: 0xff,
                g: 0xff,
                b: 0xff,
                a: 0xff,
            },
            font_size: 16.0,
        }
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }
}

impl<'a, U> View<U> for Label<'a> {
    fn render(&self, renderer: &mut dyn Renderer) {
        renderer.label(self);
    }
}

/// Something that will trigger an update when it is clicked.
pub struct Clickable<'a, U> {
    pub content: ViewObject<'a, U>,
    pub update: Option<fn() -> U>,
}

impl<'a, U> Clickable<'a, U> {
    pub fn new(content: impl View<U> + 'a) -> Self {
        Self {
            content: ViewObject::new(content),
            update: None,
        }
    }

    pub fn on_click(mut self, update: fn() -> U) -> Self {
        self.update = Some(update);
        self
    }
}

impl<'a, U> View<U> for Clickable<'a, U> {
    fn handle_input(
        &mut self,
        updates: &mut alloc::vec::Vec<U>,
        event: &InputEvent,
        captured: &mut bool,
        bounds: Aabb2D<f32>,
        mouse_pos: Xy<f32>,
    ) {
        self.content
            .as_view_mut()
            .handle_input(updates, event, captured, bounds, mouse_pos);

        if *captured {
            return;
        }

        match event {
            InputEvent::MouseButtonDown(MouseButton::Primary) => {
                if !bounds.contains(mouse_pos) {
                    return;
                }

                *captured = true;

                if let Some(update) = &self.update {
                    updates.push(update());
                }
            }
            _ => {}
        }
    }

    fn render(&self, renderer: &mut dyn Renderer) {
        self.content.as_view().render(renderer);
    }
}

pub trait AsClickable<'a, U> {
    fn on_click(self, update: fn() -> U) -> Clickable<'a, U>
    where
        Self: Sized;
}

impl<'a, T: View<U> + 'a, U> AsClickable<'a, U> for T {
    fn on_click(self, update: fn() -> U) -> Clickable<'a, U>
    where
        Self: Sized,
    {
        Clickable::new(self).on_click(update)
    }
}



#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn on_click_syntax() {
        let _label: Clickable<'_, ()> = Label::new("Something").on_click(|| ());
    }

    #[test]
    fn input_event_handling_basics() {
        let mut label: Clickable<'_, u8> = Label::new("Click Me").on_click(|| 43);

        let mut updates = vec![];
        let event = InputEvent::MouseButtonDown(MouseButton::Primary);
        let mut captured = false;
        let bounds = Aabb2D {
            x_min: 0.0,
            x_max: 5.0,
            y_min: 0.0,
            y_max: 5.0,
        };
        let mouse_pos = Xy { x: 4.1, y: 3.7 };

        label.handle_input(&mut updates, &event, &mut captured, bounds, mouse_pos);

        assert!(captured);
        assert!(updates.first().is_some_and(|u| *u == 43));
    }
}
