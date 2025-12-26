
use core::{ffi::CStr, ptr};



pub fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}



pub fn access(path: &CStr, amode: i32) -> i32 {
    unsafe { libc::access(path.as_ptr(), amode) }
}

pub fn chmod(path: &CStr, mode: u32) -> i32 {
    unsafe { libc::chmod(path.as_ptr(), mode) }
}

pub fn chown(path: &CStr, uid: u32, gid: u32) -> i32 {
    unsafe { libc::chown(path.as_ptr(), uid, gid) }
}

pub fn close(fd: i32) -> i32 {
    unsafe { libc::close(fd) }
}

pub fn dup(oldfd: i32) -> i32 {
    unsafe { libc::dup(oldfd) }
}

pub fn dup2(src: i32, dst: i32) -> i32 {
    unsafe { libc::dup2(src, dst) }
}

pub fn exit(status: i32) -> ! {
    unsafe { libc::exit(status) }
}

pub fn _exit(status: i32) -> ! {
    unsafe { libc::_exit(status) }
}

pub fn fchmod(fd: i32, mode: u32) -> i32 {
    unsafe { libc::fchmod(fd, mode) }
}

pub fn fchown(fd: i32, owner: u32, group: u32) -> i32 {
    unsafe { libc::fchown(fd, owner, group) }
}

pub fn fork() -> i32 {
    unsafe { libc::fork() }
}

pub fn getegid() -> u32 {
    unsafe { libc::getegid() }
}

pub fn geteuid() -> u32 {
    unsafe { libc::geteuid() }
}

pub fn getpid() -> i32 {
    unsafe { libc::getpid() }
}

pub fn getpgid(pid: i32) -> i32 {
    unsafe { libc::getpgid(pid) }
}

pub fn getpgrp() -> i32 {
    unsafe { libc::getpgrp() }
}

pub fn getppid() -> i32 {
    unsafe { libc::getppid() }
}

pub fn getsid(pid: i32) -> i32 {
    unsafe { libc::getsid(pid) }
}

pub fn gettid() -> i32 {
    unsafe { libc::gettid() }
}

pub fn getuid() -> u32 {
    unsafe { libc::getuid() }
}

pub fn kill(pid: i32, sig: i32) -> i32 {
    unsafe { libc::kill(pid, sig) }
}

pub fn mkdir(path: &CStr, mode: u32) -> i32 {
    unsafe { libc::mkdir(path.as_ptr(), mode) }
}

pub fn mount(source: &CStr, target: &CStr, fs: &CStr, flags: u64, data: Option<&CStr>) -> i32 {
    unsafe {
        libc::mount(
            source.as_ptr(),
            target.as_ptr(),
            fs.as_ptr(),
            flags,
            data.map_or(ptr::null(), |s| s.as_ptr() as _),
        )
    }
}

pub fn open(path: &CStr, flags: i32) -> i32 {
    unsafe { libc::open(path.as_ptr(), flags) }
}

pub fn read(fd: i32, buf: &mut [u8], count: usize) -> isize {
    unsafe { libc::read(fd, buf.as_mut_ptr() as _, count) }
}

pub fn setfsgid(gid: u32) -> i32 {
    unsafe { libc::setfsgid(gid) }
}

pub fn setfsuid(uid: u32) -> i32 {
    unsafe { libc::setfsuid(uid) }
}

pub fn setgid(gid: u32) -> i32 {
    unsafe { libc::setgid(gid) }
}

pub fn setpgid(pid: i32, pgid: i32) -> i32 {
    unsafe { libc::setpgid(pid, pgid) }
}

pub fn setregid(rgid: u32, egid: u32) -> i32 {
    unsafe { libc::setregid(rgid, egid) }
}

pub fn setresgid(rgid: u32, egid: u32, sgid: u32) -> i32 {
    unsafe { libc::setresgid(rgid, egid, sgid) }
}

pub fn setresuid(ruid: u32, euid: u32, suid: u32) -> i32 {
    unsafe { libc::setresuid(ruid, euid, suid) }
}

pub fn setreuid(ruid: u32, euid: u32) -> i32 {
    unsafe { libc::setreuid(ruid, euid) }
}

pub fn setsid() -> i32 {
    unsafe { libc::setsid() }
}

pub fn setuid(uid: u32) -> i32 {
    unsafe { libc::setuid(uid) }
}

pub fn umask(mask: u32) -> u32 {
    unsafe { libc::umask(mask) }
}

pub fn wait(status: &mut i32) -> i32 {
    unsafe { libc::wait(status as _) }
}

pub fn write(fd: i32, buf: &[u8], count: usize) -> isize {
    unsafe { libc::write(fd, buf.as_ptr() as _, count) }
}
