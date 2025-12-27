//! # Init System

use kernel::{
    c_str::NULL_CSTR,
    mount::{MountError, mount},
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

    if let Err(_mount_error) = setup_mount_points() {
        println!("ERROR (init): Failed to mount filesystems");
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
                Signal::CHLD => {
                    // use WaitStatus::{Exited, Signaled};
                    // 'reap_terminated_children: loop {
                    //     if let Ok(Exited(pid, _)) | Ok(Signaled(pid, _, _)) = pid.wait(status) {
                    //         todo!()
                    //     } else {
                    //         break 'reap_terminated_children;
                    //     }
                    // }
                }
                _ => {}
            }
        }
    }
}



fn setup_mount_points() -> Result<(), MountError> {
    use kernel::mount::{NODEV, NOEXEC, NOSUID};

    mount(c"proc",  c"/proc",    c"proc",     NOSUID | NOEXEC | NODEV, NULL_CSTR)?;
    mount(c"sys",   c"/sys",     c"sysfs",    NOSUID | NOEXEC | NODEV, NULL_CSTR)?;
    mount(c"dev",   c"/dev",     c"devtmpfs", NOSUID,                  Some(c"mode=755"))?;
    mount(c"tmpfs", c"/dev/shm", c"tmpfs",    NOSUID | NOEXEC | NODEV, Some(c"mode=1777"))?;

    Ok(())
}
