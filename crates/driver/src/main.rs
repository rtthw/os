//! # Application Driver

use {
    abi::*,
    anyhow::{Result, bail},
    kernel::shm::{Mutex, SharedMemory},
    std::sync::atomic::{AtomicU8, Ordering},
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
    map_ptr = unsafe { map_ptr.add(8) };

    // Wait for the shell to initialize the map.
    while is_map_initialized.load(Ordering::Relaxed) != 1 {}

    let mutex: Mutex<DriverInput> = unsafe { Mutex::from_existing(map_ptr) }?;

    for _i in 1..=5 {
        {
            let guard = mutex.lock()?;
            let input = unsafe { &**guard };
            println!("(driver @ {}) PONG #{}", input.id, input.events);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(())
}
