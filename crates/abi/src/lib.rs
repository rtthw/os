//! # Application Binary Interface (ABI)

pub mod elf;
pub mod layout;
pub mod path;
pub mod stable_string;
pub mod stable_vec;
pub mod type_map;

pub use {
    path::Path,
    stable_string::StableString,
    stable_vec::StableVec,
    type_map::{TypeMap, TypeMapEntry},
};

use core::{
    any::Any,
    ops::{Deref, DerefMut, Sub},
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
