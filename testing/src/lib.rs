//! # Testing Application



#[unsafe(export_name = "main")]
extern "C" fn main() {
    unsafe {
        shell::__shell_info("WORKS");
    }
}

#[allow(non_snake_case)]
pub mod shell {
    unsafe extern "Rust" {
        pub fn __shell_info(text: &str);
    }
}
