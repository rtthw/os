//! # Example Program

#![no_std]

extern crate example_dep;
extern crate time;

use core::time::Duration;

use example_dep::exit;

pub extern "C" fn main() -> ! {
    if !time::monotonic_clock_ready() {
        panic!("CLOCK NOT READY");
    }

    let start = time::now();
    while time::now().duration_since(start) < Duration::from_secs(10) {
        core::hint::spin_loop();
    }

    exit()
}
