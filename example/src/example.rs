//! # Example Program

#![no_std]

extern crate example_dep;
extern crate time;

use example_dep::exit;

pub extern "C" fn main() -> ! {
    if !time::monotonic_clock_ready() {
        panic!("CLOCK NOT READY");
    }

    for _ in 0..500 {
        pit::sleep(10_000);
    }

    exit()
}
