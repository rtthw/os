//! # Init System

use kernel::{
    Result,
    c_str::NULL_CSTR,
    file::File,
    mount::mount,
    proc::Process,
    raw::{chdir, chroot, close, exit, fork, mkdir, setsid},
    signal::{Signal, SignalMask},
};



fn main() {
    let pid = Process::current();
    if pid != 1 {
        exit(-1)
    }

    println!("\x1b[2minit\x1b[0m: Mounting filesystems...");

    if let Err(error) = setup_mount_points() {
        println!("\x1b[31mERROR\x1b[0m \x1b[2m(init)\x1b[0m: Failed to mount filesystems: {error}");
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
            "\n\x1b[33mWARN\x1b[0m \x1b[2m(init)\x1b[0m: Failed to read filesystem: {error}\n",
        );
    } else {
        println!();
    }

    println!("\x1b[2minit\x1b[0m: Starting main loop...");

    let mut shell = match std::process::Command::new("/sbin/shell").spawn() {
        Ok(child) => child,
        Err(error) => {
            println!("\x1b[31mERROR\x1b[0m \x1b[2m(init)\x1b[0m: Failed to start shell: {error}");
            exit(-1)
        }
    };

    loop {
        if let Ok(sig) = mask.wait() {
            match sig {
                Signal::CHLD => handle_sigchld(&mut shell),
                _ => {}
            }
        }
    }
}



fn setup_mount_points() -> Result<()> {
    use kernel::mount::{MOVE, MountFlags, NODEV, NOEXEC, NOSUID};

    _ = mkdir(c"/proc", 0);
    mount(
        c"proc",
        c"/proc",
        c"proc",
        NOSUID | NOEXEC | NODEV,
        NULL_CSTR,
    )?;

    _ = mkdir(c"/sys", 0);
    mount(
        c"sys",
        c"/sys",
        c"sysfs",
        NOSUID | NOEXEC | NODEV,
        NULL_CSTR,
    )?;

    _ = mkdir(c"/dev", 0);
    mount(c"dev", c"/dev", c"devtmpfs", NOSUID, Some(c"mode=755"))?;

    // mount(c"tmpfs", c"/dev/shm", c"tmpfs",    NOSUID | NOEXEC | NODEV,
    // Some(c"mode=1777"))?;

    println!("\x1b[2minit\x1b[0m: Mounting root directory...");

    if mkdir(c"/rootfs", 0) < 0 {
        return Err(kernel::Error::latest());
    }

    mount(
        c"/dev/sda1",
        c"/rootfs",
        c"vfat",
        MountFlags::default(),
        NULL_CSTR,
    )?;

    // if umount2(c"/dev", DETACH) < 0 {
    //     println!("FAILED DEV MOVE");
    //     return Err(kernel::Error::latest());
    // }

    println!("\x1b[2minit\x1b[0m: Moving dev...");
    _ = mkdir(c"/rootfs/dev", 0);
    mount(c"/dev", c"/rootfs/dev", c"", MOVE, NULL_CSTR)?;

    println!("\x1b[2minit\x1b[0m: Moving proc...");
    _ = mkdir(c"/rootfs/proc", 0);
    mount(c"/proc", c"/rootfs/proc", c"", MOVE, NULL_CSTR)?;

    println!("\x1b[2minit\x1b[0m: Moving sys...");
    _ = mkdir(c"/rootfs/sys", 0);
    mount(c"/sys", c"/rootfs/sys", c"", MOVE, NULL_CSTR)?;

    if chdir(c"rootfs") < 0 {
        return Err(kernel::Error::latest());
    }

    let old_root_fd = File::open(c"/", kernel::file::O_RDONLY)?;

    mount(c"/rootfs", c"/", c"", kernel::mount::MOVE, NULL_CSTR)?;

    if chroot(c".") < 0 {
        return Err(kernel::Error::latest());
    }

    if chdir(c"/") < 0 {
        return Err(kernel::Error::latest());
    }

    match fork() {
        0 => {
            cleanup_initramfs(old_root_fd)?;
            exit(0)
        }
        -1 => Err(kernel::Error::latest()),
        _ => {
            if close(unsafe { old_root_fd.into_raw() }) < 0 {
                Err(kernel::Error::latest())
            } else {
                Ok(())
            }
        }
    }
}

fn cleanup_initramfs(old_root_fd: File) -> Result<()> {
    let _dirfd = unsafe { old_root_fd.into_raw() };

    // TODO

    Ok(())
}

fn print_filesystem() -> Result<()> {
    const IGNORE: &[&str] = &["/proc", "/sys/class", "/sys/kernel/slab", "/sys/devices"];

    fn inner(dir: &str, depth: usize) -> Result<()> {
        for entry in std::fs::read_dir(dir).map_err(|_| kernel::Error::NOENT)? {
            let entry = entry.map_err(|_| kernel::Error::NOENT)?;
            let path = entry.path().to_str().unwrap().to_string();
            let name = path
                .rsplit_once('/')
                .map_or(path.clone(), |(_, name)| name.to_string());

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

fn handle_sigchld(shell: &mut std::process::Child) {
    use kernel::proc::{WaitStatus, wait_for_children_once};

    'reap_terminated_children: loop {
        if let Ok(status) = wait_for_children_once() {
            let termination: Option<(Process, i32)> = match status {
                WaitStatus::Exited { proc, code } => Some((proc, code)),
                WaitStatus::Signaled {
                    proc,
                    sig,
                    core_dumped: _,
                } => Some((
                    proc,
                    sig as i32 + 128, // Signal to exit code conversion.
                )),
                _ => None,
            };
            if let Some((proc, exit_code)) = termination {
                if proc == shell.id() as i32 {
                    println!(
                        "\n\x1b[33mWARN\x1b[0m \x1b[2m(init)\x1b[0m: \
                        Shell exited with code {exit_code}, restarting\n",
                    );
                    *shell = match std::process::Command::new("/sbin/shell").spawn() {
                        Ok(child) => child,
                        Err(error) => {
                            println!(
                                "\x1b[31mERROR\x1b[0m \x1b[2m(init)\x1b[0m: \
                                Failed to restart shell: {error}",
                            );
                            exit(-1)
                        }
                    };
                }
            }
        } else {
            break 'reap_terminated_children;
        }
    }
}
