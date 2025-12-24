//! # Init System

use kernel::{c_str::NULL_CSTR, mount::{MountError, mount}, proc::Process, raw::{exit, setsid, umask}};



fn main() {
    if Process::current() != 1 {
        exit(-1)
    }

    if let Err(_mount_error) = setup_mount_points() {
        exit(-1)
    }

    loop {
        match update() {
            Ok(after_update) => {
                match after_update {
                    AfterUpdate::Exit => exit(0),
                }
            }
            Err(exit_status) => exit(exit_status),
        }
    }
}

fn update() -> Result<AfterUpdate, i32> {
    let mut after_update = None;
    while after_update.is_none() {
        after_update = Some(AfterUpdate::Exit);
    }

    Ok(after_update.unwrap())
}

enum AfterUpdate {
    Exit,
}



fn setup_mount_points() -> Result<(), MountError> {
    use kernel::mount::{NODEV, NOEXEC, NOSUID};

    mount(c"proc",  c"/proc",    c"proc",     NOSUID | NOEXEC | NODEV, NULL_CSTR)?;
    mount(c"sys",   c"/sys",     c"sysfs",    NOSUID | NOEXEC | NODEV, NULL_CSTR)?;
    mount(c"dev",   c"/dev",     c"devtmpfs", NOSUID,                  Some(c"mode=755"))?;
    mount(c"tmpfs", c"/dev/shm", c"tmpfs",    NOSUID | NOEXEC | NODEV, Some(c"mode=1777"))?;

    Ok(())
}
