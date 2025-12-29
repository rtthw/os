//! # Init System

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

    println!("\x1b[2minit\x1b[0m: Mounting filesystems...");

    if let Err(error) = setup_mount_points() {
        println!("\x1b[33mERROR\x1b[0m \x1b[2m(init)\x1b[0m: Failed to mount filesystems: {error}");
        exit(-1)
    }

    // Make sure this process is a session leader.
    _ = setsid();

    println!("\x1b[2minit\x1b[0m: Blocking signals...");

    let mask = SignalMask::all();
    mask.block().unwrap();

    println!("\x1b[2minit\x1b[0m: Scanning filesystem...");

    if let Err(error) = print_filesystem() {
        println!(
            "\n\x1b[33mERROR\x1b[0m \x1b[2m(init)\x1b[0m: Failed to read filesystem: {error}\n",
        );
    } else {
        println!();
    }

    println!("\x1b[2minit\x1b[0m: Starting main loop...");

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

fn print_filesystem() -> Result<()> {
    const IGNORE: &[&str] = &[
        "/proc",
        "/sys/class",
        "/sys/kernel/slab",
        "/sys/devices",
    ];

    fn inner(dir: &str, depth: usize) -> Result<()> {
        for entry in std::fs::read_dir(dir).map_err(|_| kernel::Error::NOENT)? {
            let entry = entry.map_err(|_| kernel::Error::NOENT)?;
            let path = entry.path().to_str().unwrap().to_string();
            let name = path.rsplit_once('/').map_or(path.clone(), |(_, name)| name.to_string());

            println!("{}\x1b[2m/\x1b[0m{}", "    ".repeat(depth), name);

            if entry.file_type().map_err(|_| kernel::Error::BADF)?.is_dir() {
                if IGNORE.contains(&path.as_str()) {
                    println!("{}\x1b[2m/...\x1b[0m", "    ".repeat(depth + 1));
                    return Ok(());
                }

                inner(&path, depth + 1)?;
            }
        }

        Ok(())
    }

    println!("\x1b[2m/\x1b[0m");

    inner("/", 1)
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
