//! # Application Driver

use {
    anyhow::{Result, bail},
    kernel::shm::SharedMemory,
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

    let mutex = unsafe { kernel::shm::Mutex::from_existing(map_ptr) }?;

    for i in 1..=5 {
        {
            let _guard = mutex.lock()?;
            println!("(driver) PONG #{i}");
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(())
}
