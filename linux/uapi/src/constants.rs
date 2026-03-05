//! # Constants

#![allow(non_snake_case, overflowing_literals)]

use core::ffi::{c_int, c_ulong, c_void};

pub const STDIN_FILENO: c_int = 0;
pub const STDOUT_FILENO: c_int = 1;
pub const STDERR_FILENO: c_int = 2;

pub const EPERM: c_int = 1;
pub const ENOENT: c_int = 2;
pub const ESRCH: c_int = 3;
pub const EINTR: c_int = 4;
pub const EIO: c_int = 5;
pub const ENXIO: c_int = 6;
pub const E2BIG: c_int = 7;
pub const ENOEXEC: c_int = 8;
pub const EBADF: c_int = 9;
pub const ECHILD: c_int = 10;
pub const EAGAIN: c_int = 11;
pub const ENOMEM: c_int = 12;
pub const EACCES: c_int = 13;
pub const EFAULT: c_int = 14;
pub const ENOTBLK: c_int = 15;
pub const EBUSY: c_int = 16;
pub const EEXIST: c_int = 17;
pub const EXDEV: c_int = 18;
pub const ENODEV: c_int = 19;
pub const ENOTDIR: c_int = 20;
pub const EISDIR: c_int = 21;
pub const EINVAL: c_int = 22;
pub const ENFILE: c_int = 23;
pub const EMFILE: c_int = 24;
pub const ENOTTY: c_int = 25;
pub const ETXTBSY: c_int = 26;
pub const EFBIG: c_int = 27;
pub const ENOSPC: c_int = 28;
pub const ESPIPE: c_int = 29;
pub const EROFS: c_int = 30;
pub const EMLINK: c_int = 31;
pub const EPIPE: c_int = 32;
pub const EDOM: c_int = 33;
pub const ERANGE: c_int = 34;
pub const EWOULDBLOCK: c_int = EAGAIN;
pub const EDEADLOCK: c_int = 35;
pub const EDEADLK: c_int = 35;
pub const ENAMETOOLONG: c_int = 36;
pub const ENOLCK: c_int = 37;
pub const ENOSYS: c_int = 38;
pub const ENOTEMPTY: c_int = 39;
pub const ELOOP: c_int = 40;
pub const ENOMSG: c_int = 42;
pub const EIDRM: c_int = 43;
pub const ECHRNG: c_int = 44;
pub const EL2NSYNC: c_int = 45;
pub const EL3HLT: c_int = 46;
pub const EL3RST: c_int = 47;
pub const ELNRNG: c_int = 48;
pub const EUNATCH: c_int = 49;
pub const ENOCSI: c_int = 50;
pub const EL2HLT: c_int = 51;
pub const EBADE: c_int = 52;
pub const EBADR: c_int = 53;
pub const EXFULL: c_int = 54;
pub const ENOANO: c_int = 55;
pub const EBADRQC: c_int = 56;
pub const EBADSLT: c_int = 57;
pub const EBFONT: c_int = 59;
pub const ENOSTR: c_int = 60;
pub const ENODATA: c_int = 61;
pub const ETIME: c_int = 62;
pub const ENOSR: c_int = 63;
pub const ENONET: c_int = 64;
pub const ENOPKG: c_int = 65;
pub const EREMOTE: c_int = 66;
pub const ENOLINK: c_int = 67;
pub const EADV: c_int = 68;
pub const ESRMNT: c_int = 69;
pub const ECOMM: c_int = 70;
pub const EPROTO: c_int = 71;
pub const EMULTIHOP: c_int = 72;
pub const EDOTDOT: c_int = 73;
pub const EBADMSG: c_int = 74;
pub const EOVERFLOW: c_int = 75;
pub const ENOTUNIQ: c_int = 76;
pub const EBADFD: c_int = 77;
pub const EREMCHG: c_int = 78;
pub const ELIBACC: c_int = 79;
pub const ELIBBAD: c_int = 80;
pub const ELIBSCN: c_int = 81;
pub const ELIBMAX: c_int = 82;
pub const ELIBEXEC: c_int = 83;
pub const EILSEQ: c_int = 84;
pub const ERESTART: c_int = 85;
pub const ESTRPIPE: c_int = 86;
pub const EUSERS: c_int = 87;
pub const ENOTSOCK: c_int = 88;
pub const EDESTADDRREQ: c_int = 89;
pub const EMSGSIZE: c_int = 90;
pub const EPROTOTYPE: c_int = 91;
pub const ENOPROTOOPT: c_int = 92;
pub const EPROTONOSUPPORT: c_int = 93;
pub const ESOCKTNOSUPPORT: c_int = 94;
pub const EOPNOTSUPP: c_int = 95;
pub const EPFNOSUPPORT: c_int = 96;
pub const EAFNOSUPPORT: c_int = 97;
pub const EADDRINUSE: c_int = 98;
pub const EADDRNOTAVAIL: c_int = 99;
pub const ENETDOWN: c_int = 100;
pub const ENETUNREACH: c_int = 101;
pub const ENETRESET: c_int = 102;
pub const ECONNABORTED: c_int = 103;
pub const ECONNRESET: c_int = 104;
pub const ENOBUFS: c_int = 105;
pub const EISCONN: c_int = 106;
pub const ENOTCONN: c_int = 107;
pub const ESHUTDOWN: c_int = 108;
pub const ETOOMANYREFS: c_int = 109;
pub const ETIMEDOUT: c_int = 110;
pub const ECONNREFUSED: c_int = 111;
pub const EHOSTDOWN: c_int = 112;
pub const EHOSTUNREACH: c_int = 113;
pub const EALREADY: c_int = 114;
pub const EINPROGRESS: c_int = 115;
pub const ESTALE: c_int = 116;
pub const EUCLEAN: c_int = 117;
pub const ENOTNAM: c_int = 118;
pub const ENAVAIL: c_int = 119;
pub const EISNAM: c_int = 120;
pub const EREMOTEIO: c_int = 121;
pub const EDQUOT: c_int = 122;
pub const ENOMEDIUM: c_int = 123;
pub const EMEDIUMTYPE: c_int = 124;
pub const ECANCELED: c_int = 125;
pub const ENOKEY: c_int = 126;
pub const EKEYEXPIRED: c_int = 127;
pub const EKEYREVOKED: c_int = 128;
pub const EKEYREJECTED: c_int = 129;
pub const EOWNERDEAD: c_int = 130;
pub const ENOTRECOVERABLE: c_int = 131;
pub const ERFKILL: c_int = 132;
pub const EHWPOISON: c_int = 133;

pub const MS_ASYNC: c_int = 0x0001;
pub const MS_INVALIDATE: c_int = 0x0002;
pub const MS_SYNC: c_int = 0x0004;

pub const MS_RDONLY: c_ulong = 0x01;
pub const MS_NOSUID: c_ulong = 0x02;
pub const MS_NODEV: c_ulong = 0x04;
pub const MS_NOEXEC: c_ulong = 0x08;
pub const MS_SYNCHRONOUS: c_ulong = 0x10;
pub const MS_REMOUNT: c_ulong = 0x20;
pub const MS_MANDLOCK: c_ulong = 0x40;
pub const MS_DIRSYNC: c_ulong = 0x80;
pub const MS_NOSYMFOLLOW: c_ulong = 0x100;
pub const MS_NOATIME: c_ulong = 0x0400;
pub const MS_NODIRATIME: c_ulong = 0x0800;
pub const MS_BIND: c_ulong = 0x1000;
pub const MS_MOVE: c_ulong = 0x2000;
pub const MS_REC: c_ulong = 0x4000;
pub const MS_SILENT: c_ulong = 0x8000;
pub const MS_POSIXACL: c_ulong = 0x010000;
pub const MS_UNBINDABLE: c_ulong = 0x020000;
pub const MS_PRIVATE: c_ulong = 0x040000;
pub const MS_SLAVE: c_ulong = 0x080000;
pub const MS_SHARED: c_ulong = 0x100000;
pub const MS_RELATIME: c_ulong = 0x200000;
pub const MS_KERNMOUNT: c_ulong = 0x400000;
pub const MS_I_VERSION: c_ulong = 0x800000;
pub const MS_STRICTATIME: c_ulong = 0x1000000;
pub const MS_LAZYTIME: c_ulong = 0x2000000;
pub const MS_ACTIVE: c_ulong = 0x40000000;
pub const MS_MGC_VAL: c_ulong = 0xc0ed0000;
pub const MS_MGC_MSK: c_ulong = 0xffff0000;

pub const O_RDONLY: c_int = 0;
pub const O_WRONLY: c_int = 1;
pub const O_RDWR: c_int = 2;

pub const O_LARGEFILE: c_int = 0;
pub const O_CREAT: c_int = 0x40;
pub const O_EXCL: c_int = 0x80;
pub const O_NOCTTY: c_int = 0x100;
pub const O_TRUNC: c_int = 0x200;
pub const O_APPEND: c_int = 0x400;
pub const O_NONBLOCK: c_int = 0x800;
pub const O_NDELAY: c_int = 0x800;
pub const O_DSYNC: c_int = 0x1000;
pub const O_ASYNC: c_int = 0x2000;
pub const O_DIRECT: c_int = 0x4000;
pub const O_DIRECTORY: c_int = 0x10000;
pub const O_NOFOLLOW: c_int = 0x20000;
pub const O_NOATIME: c_int = 0x40000;
pub const O_CLOEXEC: c_int = 0x80000;
pub const O_PATH: c_int = 0x200000;
pub const O_TMPFILE: c_int = 0x400000 | O_DIRECTORY;
pub const O_SYNC: c_int = 0x101000;
pub const O_RSYNC: c_int = O_SYNC;
pub const O_FSYNC: c_int = O_SYNC;

pub const PROT_NONE: c_int = 0;
pub const PROT_READ: c_int = 1;
pub const PROT_WRITE: c_int = 2;
pub const PROT_EXEC: c_int = 4;

pub const AT_FDCWD: c_int = -100;
pub const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
pub const AT_REMOVEDIR: c_int = 0x200;
pub const AT_SYMLINK_FOLLOW: c_int = 0x400;
pub const AT_NO_AUTOMOUNT: c_int = 0x800;
pub const AT_EMPTY_PATH: c_int = 0x1000;
pub const AT_RECURSIVE: c_int = 0x8000;

pub const MAP_FILE: c_int = 0x0000;
pub const MAP_SHARED: c_int = 0x0001;
pub const MAP_PRIVATE: c_int = 0x0002;
pub const MAP_FIXED: c_int = 0x0010;
pub const MAP_FAILED: *mut c_void = !0 as *mut c_void;

pub const RTLD_LOCAL: c_int = 0;
pub const RTLD_LAZY: c_int = 1;

pub const EPOLL_CTL_ADD: c_int = 1;
pub const EPOLL_CTL_DEL: c_int = 2;
pub const EPOLL_CTL_MOD: c_int = 3;

pub const EPOLLIN: c_int = 0x1;
pub const EPOLLPRI: c_int = 0x2;
pub const EPOLLOUT: c_int = 0x4;
pub const EPOLLERR: c_int = 0x8;
pub const EPOLLHUP: c_int = 0x10;
pub const EPOLLRDNORM: c_int = 0x40;
pub const EPOLLRDBAND: c_int = 0x80;
pub const EPOLLWRNORM: c_int = 0x100;
pub const EPOLLWRBAND: c_int = 0x200;
pub const EPOLLMSG: c_int = 0x400;
pub const EPOLLRDHUP: c_int = 0x2000;
pub const EPOLLEXCLUSIVE: c_int = 0x10000000;
pub const EPOLLWAKEUP: c_int = 0x20000000;
pub const EPOLLONESHOT: c_int = 0x40000000;
pub const EPOLLET: c_int = 0x80000000;

pub const WNOHANG: c_int = 0x00000001;
pub const WUNTRACED: c_int = 0x00000002;
pub const WSTOPPED: c_int = WUNTRACED;
pub const WEXITED: c_int = 0x00000004;
pub const WCONTINUED: c_int = 0x00000008;
pub const WNOWAIT: c_int = 0x01000000;

pub const fn WIFSTOPPED(status: c_int) -> bool {
    (status & 0xff) == 0x7f
}

pub const fn WSTOPSIG(status: c_int) -> c_int {
    (status >> 8) & 0xff
}

pub const fn WIFCONTINUED(status: c_int) -> bool {
    status == 0xffff
}

pub const fn WIFSIGNALED(status: c_int) -> bool {
    ((status & 0x7f) + 1) as i8 >= 2
}

pub const fn WTERMSIG(status: c_int) -> c_int {
    status & 0x7f
}

pub const fn WIFEXITED(status: c_int) -> bool {
    (status & 0x7f) == 0
}

pub const fn WEXITSTATUS(status: c_int) -> c_int {
    (status >> 8) & 0xff
}

pub const fn WCOREDUMP(status: c_int) -> bool {
    (status & 0x80) != 0
}

pub const TCGETS: c_ulong = 0x5401;
pub const TCSETS: c_ulong = 0x5402;
pub const TCSETSW: c_ulong = 0x5403;
pub const TCSETSF: c_ulong = 0x5404;
pub const TCGETA: c_ulong = 0x5405;
pub const TCSETA: c_ulong = 0x5406;
pub const TCSETAW: c_ulong = 0x5407;
pub const TCSETAF: c_ulong = 0x5408;
pub const TCSBRK: c_ulong = 0x5409;
pub const TCXONC: c_ulong = 0x540A;
pub const TCFLSH: c_ulong = 0x540B;
pub const TIOCEXCL: c_ulong = 0x540C;
pub const TIOCNXCL: c_ulong = 0x540D;
pub const TIOCSCTTY: c_ulong = 0x540E;
pub const TIOCGPGRP: c_ulong = 0x540F;
pub const TIOCSPGRP: c_ulong = 0x5410;
pub const TIOCOUTQ: c_ulong = 0x5411;
pub const TIOCSTI: c_ulong = 0x5412;
pub const TIOCGWINSZ: c_ulong = 0x5413;
pub const TIOCSWINSZ: c_ulong = 0x5414;
pub const TIOCMGET: c_ulong = 0x5415;
pub const TIOCMBIS: c_ulong = 0x5416;
pub const TIOCMBIC: c_ulong = 0x5417;
pub const TIOCMSET: c_ulong = 0x5418;
pub const TIOCGSOFTCAR: c_ulong = 0x5419;
pub const TIOCSSOFTCAR: c_ulong = 0x541A;
pub const FIONREAD: c_ulong = 0x541B;
pub const TIOCINQ: c_ulong = FIONREAD;
pub const TIOCLINUX: c_ulong = 0x541C;
pub const TIOCCONS: c_ulong = 0x541D;
pub const TIOCGSERIAL: c_ulong = 0x541E;
pub const TIOCSSERIAL: c_ulong = 0x541F;
pub const TIOCPKT: c_ulong = 0x5420;
pub const FIONBIO: c_ulong = 0x5421;
pub const TIOCNOTTY: c_ulong = 0x5422;
pub const TIOCSETD: c_ulong = 0x5423;
pub const TIOCGETD: c_ulong = 0x5424;
pub const TCSBRKP: c_ulong = 0x5425;
pub const TIOCSBRK: c_ulong = 0x5427;
pub const TIOCCBRK: c_ulong = 0x5428;
pub const TIOCGSID: c_ulong = 0x5429;
pub const TCGETS2: c_ulong = 0x802c542a;
pub const TCSETS2: c_ulong = 0x402c542b;
pub const TCSETSW2: c_ulong = 0x402c542c;
pub const TCSETSF2: c_ulong = 0x402c542d;
pub const TIOCGRS485: c_ulong = 0x542E;
pub const TIOCSRS485: c_ulong = 0x542F;
pub const TIOCGPTN: c_ulong = 0x80045430;
pub const TIOCSPTLCK: c_ulong = 0x40045431;
pub const TIOCGDEV: c_ulong = 0x80045432;
pub const TCGETX: c_ulong = 0x5432;
pub const TCSETX: c_ulong = 0x5433;
pub const TCSETXF: c_ulong = 0x5434;
pub const TCSETXW: c_ulong = 0x5435;
pub const TIOCSIG: c_ulong = 0x40045436;
pub const TIOCVHANGUP: c_ulong = 0x5437;
pub const TIOCGPKT: c_ulong = 0x80045438;
pub const TIOCGPTLCK: c_ulong = 0x80045439;
pub const TIOCGEXCL: c_ulong = 0x80045440;
pub const TIOCGPTPEER: c_ulong = 0x5441;

pub const S_IFIFO: u32 = 0o1_0000;
pub const S_IFCHR: u32 = 0o2_0000;
pub const S_IFBLK: u32 = 0o6_0000;
pub const S_IFDIR: u32 = 0o4_0000;
pub const S_IFREG: u32 = 0o10_0000;
pub const S_IFLNK: u32 = 0o12_0000;
pub const S_IFSOCK: u32 = 0o14_0000;
pub const S_IFMT: u32 = 0o17_0000;
pub const S_IRWXU: u32 = 0o0700;
pub const S_IXUSR: u32 = 0o0100;
pub const S_IWUSR: u32 = 0o0200;
pub const S_IRUSR: u32 = 0o0400;
pub const S_IRWXG: u32 = 0o0070;
pub const S_IXGRP: u32 = 0o0010;
pub const S_IWGRP: u32 = 0o0020;
pub const S_IRGRP: u32 = 0o0040;
pub const S_IRWXO: u32 = 0o0007;
pub const S_IXOTH: u32 = 0o0001;
pub const S_IWOTH: u32 = 0o0002;
pub const S_IROTH: u32 = 0o0004;

pub const SIG_BLOCK: c_int = 0;
pub const SIG_UNBLOCK: c_int = 1;
pub const SIG_SETMASK: c_int = 2;

pub const SFD_NONBLOCK: c_int = 0x800;
pub const SFD_CLOEXEC: c_int = 0x80000;

pub const PTHREAD_PROCESS_PRIVATE: c_int = 0;
pub const PTHREAD_PROCESS_SHARED: c_int = 1;
