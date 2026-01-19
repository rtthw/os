//! # Testing Application


extern crate abi;

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
pub extern "C" fn f32_id_as_u128() -> u128 {
    unsafe {
        shell::debug("abi_test_suite::shell::debug");
        std::mem::transmute(std::any::TypeId::of::<f32>())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn abi_path_id_as_u128() -> u128 {
    unsafe { std::mem::transmute(std::any::TypeId::of::<abi::Path>()) }
}
