//! # Example Program

#![no_std]

extern crate example_dep;
extern crate time;

use example_dep::exit;

const TEST_PAGE_FAULT: bool = false;
const TEST_WRITE_TIME: bool = false;

pub extern "C" fn main() -> ! {
    if TEST_PAGE_FAULT {
        let ptr = 0xab0de as *mut u8;
        unsafe {
            ptr.write(43);
        }
    }
    if TEST_WRITE_TIME {
        unsafe {
            time::set_monotonic_clock::<MaliciousClock>();
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

struct MaliciousClock;

impl time::ClockMonotonic for MaliciousClock {
    fn now() -> u64 {
        1
    }

    fn period() -> u64 {
        1
    }
}
