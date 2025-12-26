//! # Init System

use kernel::{
    c_str::NULL_CSTR,
    file::File,
    mount::{MountError, mount},
    proc::{Process, ProcessGroup, Session},
    raw::{exit, setsid},
};



fn main() {
    if Process::current() != 1 {
        exit(-1)
    }

    if let Err(_mount_error) = setup_mount_points() {
        exit(-1)
    }

    // Make sure this process is a session leader.
    _ = setsid();

    open_tty();

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

fn open_tty() {
    use kernel::file::{O_NOCTTY, O_NONBLOCK, O_RDWR};

    _ = File::STDIN.close();
    _ = File::STDOUT.close();
    _ = File::STDERR.close();

    let fd = File::open("dev/tty", O_RDWR | O_NONBLOCK | O_NOCTTY)
        .expect("failed to open 'dev/tty' as standard input");
    #[cfg(debug_assertions)]
    debug_assert_eq!(fd, File::STDIN, "failed to open 'dev/tty' as standard input");

    File::STDIN.change_mode(0o00620) // RWUSR, WGRP
        .expect("cannot change STDIN mode");
    File::STDIN.change_owner(0, 0) // root, root
        .expect("cannot change STDIN ownership");

    assert!(File::STDIN.is_a_tty(), "STDIN should be a TTY now");

    _ = File::STDIN.release_terminal_control();

    let fd = File::open("dev/tty", O_RDWR | O_NONBLOCK | O_NOCTTY)
        .expect("failed to open 'dev/tty' as standard input");
    #[cfg(debug_assertions)]
    debug_assert_eq!(fd, File::STDIN, "failed to open 'dev/tty' as standard input");

    if File::STDIN.terminal_session().is_ok_and(|session| session != Session::current()) {
        File::STDIN.take_terminal_control()
            .expect("cannot make STDIN the controlling terminal");
    }

    _ = File::STDIN.set_foreground_process_group(ProcessGroup::current());

    if !(
        File::STDIN.duplicate().is_ok_and(|fd| fd == File::STDOUT)
        && File::STDIN.duplicate().is_ok_and(|fd| fd == File::STDERR)
    ) {
        panic!("something went wrong while duping STDIN into STDOUT/STDERR");
    }
}
