//! # Testing Application



abi::manifest! {
    name: "Testing",
    entry_point: main,
    dependencies: &[],
}

extern "C" fn main() {
    unsafe {
        shell::__shell_info("WORKS");
    }
}

pub mod shell {
    unsafe extern "Rust" {
        pub fn __shell_info(text: &str);
    }
}
