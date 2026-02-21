//! # Application Binary Interface (ABI)

pub mod cursor_icon;
pub mod elf;
pub mod flex;
pub mod layout;
pub mod math;
pub mod mem;
pub mod path;
pub mod stable_string;
pub mod stable_vec;
pub mod text;
pub mod tree;
pub mod type_map;
pub mod view;

pub use {
    cursor_icon::CursorIcon,
    flex::{AxisAlignment, CrossAlignment, Flex, FlexParams},
    math::{Aabb2D, Axis, Transform2D, Xy},
    path::Path,
    stable_string::StableString,
    stable_vec::StableVec,
    text::{FontStyle, LineHeight, TextAlignment, TextWrapMode},
    type_map::{TypeMap, TypeMapEntry},
    view::*,
};

use std::fmt::Debug;

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
    pub known_bounds: Aabb2D,
    pub events: [Option<DriverInputEvent>; DRIVER_INPUT_EVENT_CAPACITY],
    pub render: Render,
}

pub const DRIVER_INPUT_EVENT_CAPACITY: usize = 16;

impl DriverInput {
    pub fn new(initial_bounds: Aabb2D) -> Self {
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
    WindowResize(Aabb2D),
}
