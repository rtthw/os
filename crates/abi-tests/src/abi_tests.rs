//! # Testing Application

extern crate abi;

use std::{any::TypeId, mem::transmute};

abi::include! {
    mod shell {
        fn debug(text: &str);
        fn error(text: &str);
        fn info(text: &str);
        fn trace(text: &str);
        fn warn(text: &str);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn id_of_f32() -> u128 {
    unsafe { transmute(TypeId::of::<f32>()) }
}

#[unsafe(no_mangle)]
pub extern "C" fn id_of_path() -> u128 {
    unsafe { transmute(TypeId::of::<abi::Path>()) }
}

#[unsafe(no_mangle)]
pub extern "C" fn id_of_dyn_element() -> u128 {
    unsafe { transmute(TypeId::of::<dyn abi::Element>()) }
}

#[unsafe(no_mangle)]
pub extern "C" fn id_of_box_dyn_element() -> u128 {
    unsafe { transmute(TypeId::of::<Box<dyn abi::Element>>()) }
}
