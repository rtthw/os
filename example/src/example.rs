//! # Example Program

#![no_std]

extern crate example_dep;
extern crate time;

use example_dep::exit;

const TEST_PAGE_FAULT: bool = false;

pub extern "C" fn main() -> ! {
    if TEST_PAGE_FAULT {
        let ptr = 0xab0de as *mut u8;
        unsafe {
            ptr.write(43);
        }
    }

    if !time::monotonic_clock_ready() {
        panic!("CLOCK NOT READY");
    }

    for _ in 0..500 {
        pit::sleep(10_000);
    }

    exit()
}
