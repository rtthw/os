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

use {alloc::boxed::Box, core::any::Any};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");



pub trait App<U: Any> {
    fn render(&mut self, bounds: Aabb2D<f32>) -> RenderPass<'_>;

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
            fn render(&mut self, bounds: Aabb2D<f32>) -> RenderPass<'_> {
                self.app.render(bounds)
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
    /// Transparently calls [`App::render`].
    fn render(&mut self, bounds: Aabb2D<f32>) -> RenderPass<'_>;
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



#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Aabb2D<V> {
    pub x_min: V,
    pub x_max: V,
    pub y_min: V,
    pub y_max: V,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Rgba<V> {
    pub r: V,
    pub g: V,
    pub b: V,
    pub a: V,
}



#[repr(C)]
pub struct RenderPass<'a> {
    pub bounds: Aabb2D<f32>,
    pub layers: Vec<RenderLayer<'a>>,
}

#[repr(C)]
pub struct RenderLayer<'a> {
    pub objects: Vec<RenderObject<'a>>,
}

#[repr(C)]
pub enum RenderObject<'a> {
    Quad {
        bounds: Aabb2D<f32>,
        color: Rgba<u8>,
    },
    Text {
        text: alloc::borrow::Cow<'a, str>,
        bounds: Aabb2D<f32>,
        color: Rgba<u8>,
        font_size: f32,
    },
    Button {
        text: alloc::borrow::Cow<'a, str>,
        on_click: fn() -> Box<dyn Any>,
    },
}
