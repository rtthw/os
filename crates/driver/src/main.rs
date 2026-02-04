//! # Application Driver

use {
    abi::*,
    anyhow::{Result, bail},
    kernel::shm::{Mutex, SharedMemory},
    std::sync::atomic::{AtomicU8, AtomicU64, Ordering},
};



fn main() -> Result<()> {
    let mut args = std::env::args();
    let _program_name = args.next();

    let Some(app_name) = args.next() else {
        bail!("no application name provided");
    };

    let map = SharedMemory::open(format!("/shmem_{}", app_name).as_str())?;
    let mut map_ptr = map.as_ptr();
    let is_map_initialized: &mut AtomicU8 = unsafe { &mut *(map_ptr as *mut AtomicU8) };
    map_ptr = unsafe { map_ptr.add(size_of::<*mut ()>()) };
    let next_input_id: &mut AtomicU64 = unsafe { &mut *(map_ptr as *mut AtomicU64) };
    map_ptr = unsafe { map_ptr.add(size_of::<*mut ()>()) };

    // Wait for the shell to initialize the map.
    while is_map_initialized.load(Ordering::Relaxed) != 1 {}

    let mutex: Mutex<DriverInput> = unsafe { Mutex::from_existing(map_ptr) }?;

    let mut drain_counts = [0; DRIVER_INPUT_EVENT_CAPACITY + 1];
    let mut seen_input_id: u64 = 0;
    'handle_input: loop {
        let input_id = next_input_id.load(Ordering::Relaxed);
        if input_id == seen_input_id {
            continue 'handle_input;
        }
        if input_id == u64::MAX {
            break 'handle_input;
        }

        let mut guard = mutex.lock()?;
        let input = unsafe { &mut **guard };

        seen_input_id = input_id;

        let mut drain_count = 0;
        for _event in input.drain_events() {
            drain_count += 1;
        }

        drain_counts[drain_count] += 1;
    }

    println!("(driver) DRAIN_COUNTS = {drain_counts:?}");

    Ok(())
}
