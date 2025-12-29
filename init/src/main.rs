//! # Init System

use std::collections::HashMap;

use kernel::{
    Result,
    c_str::NULL_CSTR,
    mount::mount,
    proc::Process,
    raw::{exit, setsid},
    signal::{Signal, SignalMask},
};



fn main() {
    let pid = Process::current();
    if pid != 1 {
        exit(-1)
    }

    println!("init: Mounting filesystems...");

    if let Err(error) = setup_mount_points() {
        println!("ERROR (init): Failed to mount filesystems: {error}");
        exit(-1)
    }

    // Make sure this process is a session leader.
    _ = setsid();

    println!("init: Blocking signals...");

    let mask = SignalMask::all();
    mask.block().unwrap();

    println!("init: Starting main loop...");

    loop {
        if let Ok(sig) = mask.wait() {
            match sig {
                Signal::CHLD => handle_sigchld(),
                _ => {}
            }
        }
    }
}



fn setup_mount_points() -> Result<()> {
    use kernel::mount::{NODEV, NOEXEC, NOSUID};

    mount(c"proc",  c"/proc",    c"proc",     NOSUID | NOEXEC | NODEV, NULL_CSTR)?;
    mount(c"sys",   c"/sys",     c"sysfs",    NOSUID | NOEXEC | NODEV, NULL_CSTR)?;
    mount(c"dev",   c"/dev",     c"devtmpfs", NOSUID,                  Some(c"mode=755"))?;
    // mount(c"tmpfs", c"/dev/shm", c"tmpfs",    NOSUID | NOEXEC | NODEV, Some(c"mode=1777"))?;

    Ok(())
}

fn handle_sigchld() {
    use kernel::proc::{WaitStatus::{Exited, Signaled}, wait_for_children_once};

    'reap_terminated_children: loop {
        if let Ok(Exited { proc, .. }) | Ok(Signaled { proc, .. }) = wait_for_children_once() {
            let _ = proc;
        } else {
            break 'reap_terminated_children;
        }
    }
}
