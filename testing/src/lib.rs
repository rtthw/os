//! # Testing Application



abi::manifest! {
    name: "Testing",
    entry_point: main,
    dependencies: &[],
}

extern "C" fn main() {
    unsafe {
        shell::debug("TEST");
        shell::error("TEST");
        shell::info("TEST");
        shell::trace("TEST");
        shell::warn("TEST");
    }
}

abi::include! {
    mod shell {
        fn debug(text: &str);
        fn error(text: &str);
        fn info(text: &str);
        fn trace(text: &str);
        fn warn(text: &str);
    }
}
