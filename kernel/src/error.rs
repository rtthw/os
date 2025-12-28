
use crate::raw;



/// Generic error type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum Error {
    NULL = 0,
    /// Operation not permitted.
    PERM = 1,
    /// No such file or directory.
    NOENT = 2,
    /// No such process.
    SRCH = 3,
    /// Interrupted function call.
    INTR = 4,
    /// Input/output error.
    IO = 5,
    /// No such device or address.
    NXIO = 6,
    /// Argument list too long.
    E2BIG = 7, // FIXME: Name cannot start with a number.
    /// Exec format error.
    NOEXEC = 8,
    /// Bad file descriptor.
    BADF = 9,
    /// No child processes.
    CHILD = 10,
    /// Resource temporarily unavailable.
    AGAIN = 11,
    /// Not enough space/cannot allocate memory.
    NOMEM = 12,
    /// Permission denied.
    ACCES = 13,
    /// Bad address.
    FAULT = 14,
    /// Block device required.
    NOTBLK = 15,
    /// Device or resource busy.
    BUSY = 16,
    /// File exists.
    EXIST = 17,
    /// Invalid cross-device link.
    XDEV = 18,
    /// No such device.
    NODEV = 19,
    /// Not a directory.
    NOTDIR = 20,
    /// Is a directory.
    ISDIR = 21,
    /// Invalid argument.
    INVAL = 22,
    /// Too many open files in system. This is probably a result of encountering the
    /// `/proc/sys/fs/file-max` limit.
    NFILE = 23,
    /// Too many open files.
    MFILE = 24,
    /// Inappropriate I/O control operation.
    NOTTY = 25,
    /// Text file busy.
    TXTBSY = 26,
    /// File too large.
    FBIG = 27,
    /// No space left on device.
    NOSPC = 28,
    /// Invalid seek.
    SPIPE = 29,
    /// Read-only filesystem.
    ROFS = 30,
    /// Too many links.
    MLINK = 31,
    /// Broken pipe.
    PIPE = 32,
    /// Mathematics argument out of domain of function.
    DOM = 33,
    /// Result too large.
    RANGE = 34,



    /// Resource deadlock avoided.
    DEADLK = 35,
    /// Filename too long.
    NAMETOOLONG = 36,
    /// No locks available.
    NOLCK = 37,
    /// Function not implemented.
    NOSYS = 38,
    /// Directory not empty.
    NOTEMPTY = 39,
    /// Too many levels of symbolic links.
    LOOP = 40,
    /// No message of the desired type.
    NOMSG = 42,
    /// Identifier removed.
    IDRM = 43,
    /// Channel number out of range.
    CHRNG = 44,
    /// Level 2 not synchronized.
    L2NSYNC = 45,
    /// Level 3 halted.
    L3HLT = 46,
    /// Level 3 reset.
    L3RST = 47,
    /// Link number out of range.
    LNRNG = 48,
    /// Protocol driver not attached.
    UNATCH = 49,
    NOCSI = 50,
    /// Level 2 halted.
    L2HLT = 51,
    /// Invalid exchange.
    BADE = 52,
    /// Invalid request descriptor.
    BADR = 53,
    /// Exchange full.
    XFULL = 54,
    /// No anode.
    NOANO = 55,
    /// Invalid request code.
    BADRQC = 56,
    /// Invalid slot.
    BADSLT = 57,
    BFONT = 59,
    /// Not a stream.
    NOSTR = 60,
    /// The named attribute does not exist, or the process has no access to this attribute.
    NODATA = 61,
    /// Timer expired.
    TIME = 62,
    /// No stream resources.
    NOSR = 63,
    /// Machine is not on the network.
    NONET = 64,
    /// Package not installed.
    NOPKG = 65,
    /// Object is remote.
    REMOTE = 66,
    /// Link has been severed.
    NOLINK = 67,
    ADV = 68,
    SRMNT = 69,
    /// Communication error on send.
    COMM = 70,
    /// Protocol error.
    PROTO = 71,
    /// Multihop attempted.
    MULTIHOP = 72,
    DOTDOT = 73,
    /// Value too large to be stored in data type.
    OVERFLOW = 75,
    /// Name not unique on network.
    NOTUNIQ = 76,
    /// File descriptor in bad state.
    BADFD = 77,
    /// Bad message.
    BADMSG = 74,
    /// Remote address changed.
    REMCHG = 78,
    /// Cannot access a needed shared library.
    LIBACC = 79,
    /// Accessing a corrupted shared library.
    LIBBAD = 80,
    /// `.lib` section in `a.out` corrupted.
    LIBSCN = 81,
    /// Attempting to link in too many shared libraries.
    LIBMAX = 82,
    /// Cannot exec a shared library directly.
    LIBEXEC = 83,
    /// Invalid or incomplete multibyte or wide character.
    ILSEQ = 84,
    /// Interrupted system call should be restarted.
    RESTART = 85,
    /// Streams pipe error.
    STRPIPE = 86,
    /// Too many users.
    USERS = 87,
    /// Not a socket.
    NOTSOCK = 88,
    /// Destination address required.
    DESTADDRREQ = 89,
    /// Message too long.
    MSGSIZE = 90,
    /// Protocol wrong type for socket.
    PROTOTYPE = 91,
    /// Protocol not available.
    NOPROTOOPT = 92,
    /// Protocol not supported.
    PROTONOSUPPORT = 93,
    /// Socket type not supported.
    SOCKTNOSUPPORT = 94,
    /// Operation not supported on socket.
    OPNOTSUPP = 95,
    /// Protocol family not supported.
    PFNOSUPPORT = 96,
    /// Address family not supported.
    AFNOSUPPORT = 97,
    /// Address already in use.
    ADDRINUSE = 98,
    /// Address not available.
    ADDRNOTAVAIL = 99,
    /// Network is down.
    NETDOWN = 100,
    /// Network unreachable.
    NETUNREACH = 101,
    /// Connection aborted by network.
    NETRESET = 102,
    /// Connection aborted.
    CONNABORTED = 103,
    /// Connection reset.
    CONNRESET = 104,
    /// No buffer space available.
    NOBUFS = 105,
    /// Socket is connected.
    ISCONN = 106,
    /// The socket is not connected.
    NOTCONN = 107,
    /// Cannot send after transport endpoint shutdown.
    SHUTDOWN = 108,
    /// Too many references: cannot splice.
    TOOMANYREFS = 109,
    /// Connection timed out.
    TIMEDOUT = 110,
    /// Connection refused.
    CONNREFUSED = 111,
    /// Host is down.
    HOSTDOWN = 112,
    /// Host is unreachable.
    HOSTUNREACH = 113,
    /// Connection already in progress.
    ALREADY = 114,
    /// Operation in progress.
    INPROGRESS = 115,
    /// Stale file handle.
    STALE = 116,
    /// Structure needs cleaning.
    UCLEAN = 117,
    NOTNAM = 118,
    NAVAIL = 119,
    /// Is a named type file.
    ISNAM = 120,
    /// Remote I/O error.
    REMOTEIO = 121,
    /// Disk quota exceeded.
    DQUOT = 122,
    /// No medium found.
    NOMEDIUM = 123,
    /// Wrong medium type.
    MEDIUMTYPE = 124,
    /// Operation canceled.
    CANCELED = 125,
    /// Required key not available.
    NOKEY = 126,
    /// Key has expired.
    KEYEXPIRED = 127,
    /// Key has been revoked.
    KEYREVOKED = 128,
    /// Key was rejected by service.
    KEYREJECTED = 129,
    /// Owner died.
    OWNERDEAD = 130,
    /// State not recoverable.
    NOTRECOVERABLE = 131,
    /// Memory page has hardware error.
    HWPOISON = 133,
    /// Operation not possible due to RF-kill.
    RFKILL = 132,
}

impl Error {
    /// Create an [`Error`] from its raw `i32` equivalent.
    pub const fn from_raw(num: i32) -> Error {
        from_raw(num)
    }

    /// Get the most recent kernel error.
    pub fn latest() -> Self {
        Self::from_raw(raw::errno())
    }

    /// Get a text description of the error.
    ///
    /// ## Example
    /// ```rust
    /// use kernel::Error;
    /// assert_eq!(Error::NOENT.description(), "No such file or directory");
    /// ```
    pub const fn description(&self) -> &'static str {
        description(*self)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.description())
    }
}

impl core::error::Error for Error {}



const fn from_raw(num: i32) -> Error {
    use Error::*;

    match num {
        libc::EPERM => PERM,
        libc::ENOENT => NOENT,
        libc::ESRCH => SRCH,
        libc::EINTR => INTR,
        libc::EIO => IO,
        libc::ENXIO => NXIO,
        libc::E2BIG => E2BIG,
        libc::ENOEXEC => NOEXEC,
        libc::EBADF => BADF,
        libc::ECHILD => CHILD,
        libc::EAGAIN => AGAIN,
        libc::ENOMEM => NOMEM,
        libc::EACCES => ACCES,
        libc::EFAULT => FAULT,
        libc::ENOTBLK => NOTBLK,
        libc::EBUSY => BUSY,
        libc::EEXIST => EXIST,
        libc::EXDEV => XDEV,
        libc::ENODEV => NODEV,
        libc::ENOTDIR => NOTDIR,
        libc::EISDIR => ISDIR,
        libc::EINVAL => INVAL,
        libc::ENFILE => NFILE,
        libc::EMFILE => MFILE,
        libc::ENOTTY => NOTTY,
        libc::ETXTBSY => TXTBSY,
        libc::EFBIG => FBIG,
        libc::ENOSPC => NOSPC,
        libc::ESPIPE => SPIPE,
        libc::EROFS => ROFS,
        libc::EMLINK => MLINK,
        libc::EPIPE => PIPE,
        libc::EDOM => DOM,
        libc::ERANGE => RANGE,
        libc::EDEADLK => DEADLK,
        libc::ENAMETOOLONG => NAMETOOLONG,
        libc::ENOLCK => NOLCK,
        libc::ENOSYS => NOSYS,
        libc::ENOTEMPTY => NOTEMPTY,
        libc::ELOOP => LOOP,
        libc::ENOMSG => NOMSG,
        libc::EIDRM => IDRM,
        libc::ECHRNG => CHRNG,
        libc::EL2NSYNC => L2NSYNC,
        libc::EL3HLT => L3HLT,
        libc::EL3RST => L3RST,
        libc::ELNRNG => LNRNG,
        libc::EUNATCH => UNATCH,
        libc::ENOCSI => NOCSI,
        libc::EL2HLT => L2HLT,
        libc::EBADE => BADE,
        libc::EBADR => BADR,
        libc::EXFULL => XFULL,
        libc::ENOANO => NOANO,
        libc::EBADRQC => BADRQC,
        libc::EBADSLT => BADSLT,
        libc::EBFONT => BFONT,
        libc::ENOSTR => NOSTR,
        libc::ENODATA => NODATA,
        libc::ETIME => TIME,
        libc::ENOSR => NOSR,
        libc::ENONET => NONET,
        libc::ENOPKG => NOPKG,
        libc::EREMOTE => REMOTE,
        libc::ENOLINK => NOLINK,
        libc::EADV => ADV,
        libc::ESRMNT => SRMNT,
        libc::ECOMM => COMM,
        libc::EPROTO => PROTO,
        libc::EMULTIHOP => MULTIHOP,
        libc::EDOTDOT => DOTDOT,
        libc::EBADMSG => BADMSG,
        libc::EOVERFLOW => OVERFLOW,
        libc::ENOTUNIQ => NOTUNIQ,
        libc::EBADFD => BADFD,
        libc::EREMCHG => REMCHG,
        libc::ELIBACC => LIBACC,
        libc::ELIBBAD => LIBBAD,
        libc::ELIBSCN => LIBSCN,
        libc::ELIBMAX => LIBMAX,
        libc::ELIBEXEC => LIBEXEC,
        libc::EILSEQ => ILSEQ,
        libc::ERESTART => RESTART,
        libc::ESTRPIPE => STRPIPE,
        libc::EUSERS => USERS,
        libc::ENOTSOCK => NOTSOCK,
        libc::EDESTADDRREQ => DESTADDRREQ,
        libc::EMSGSIZE => MSGSIZE,
        libc::EPROTOTYPE => PROTOTYPE,
        libc::ENOPROTOOPT => NOPROTOOPT,
        libc::EPROTONOSUPPORT => PROTONOSUPPORT,
        libc::ESOCKTNOSUPPORT => SOCKTNOSUPPORT,
        libc::EOPNOTSUPP => OPNOTSUPP,
        libc::EPFNOSUPPORT => PFNOSUPPORT,
        libc::EAFNOSUPPORT => AFNOSUPPORT,
        libc::EADDRINUSE => ADDRINUSE,
        libc::EADDRNOTAVAIL => ADDRNOTAVAIL,
        libc::ENETDOWN => NETDOWN,
        libc::ENETUNREACH => NETUNREACH,
        libc::ENETRESET => NETRESET,
        libc::ECONNABORTED => CONNABORTED,
        libc::ECONNRESET => CONNRESET,
        libc::ENOBUFS => NOBUFS,
        libc::EISCONN => ISCONN,
        libc::ENOTCONN => NOTCONN,
        libc::ESHUTDOWN => SHUTDOWN,
        libc::ETOOMANYREFS => TOOMANYREFS,
        libc::ETIMEDOUT => TIMEDOUT,
        libc::ECONNREFUSED => CONNREFUSED,
        libc::EHOSTDOWN => HOSTDOWN,
        libc::EHOSTUNREACH => HOSTUNREACH,
        libc::EALREADY => ALREADY,
        libc::EINPROGRESS => INPROGRESS,
        libc::ESTALE => STALE,
        libc::EUCLEAN => UCLEAN,
        libc::ENOTNAM => NOTNAM,
        libc::ENAVAIL => NAVAIL,
        libc::EISNAM => ISNAM,
        libc::EREMOTEIO => REMOTEIO,
        libc::EDQUOT => DQUOT,
        libc::ENOMEDIUM => NOMEDIUM,
        libc::EMEDIUMTYPE => MEDIUMTYPE,
        libc::ECANCELED => CANCELED,
        libc::ENOKEY => NOKEY,
        libc::EKEYEXPIRED => KEYEXPIRED,
        libc::EKEYREVOKED => KEYREVOKED,
        libc::EKEYREJECTED => KEYREJECTED,
        libc::EOWNERDEAD => OWNERDEAD,
        libc::ENOTRECOVERABLE => NOTRECOVERABLE,
        libc::ERFKILL => RFKILL,
        libc::EHWPOISON => HWPOISON,
        _ => NULL,
    }
}

const fn description(error: Error) -> &'static str {
    use Error::*;

    match error {
        NULL => "Unknown error",
        PERM => "Operation not permitted",
        NOENT => "No such file or directory",
        SRCH => "No such process",
        INTR => "Interrupted system call",
        IO => "I/O error",
        NXIO => "No such device or address",
        E2BIG => "Argument list too long",
        NOEXEC => "Exec format error",
        BADF => "Bad file number",
        CHILD => "No child processes",
        AGAIN => "Try again",
        NOMEM => "Out of memory",
        ACCES => "Permission denied",
        FAULT => "Bad address",
        NOTBLK => "Block device required",
        BUSY => "Device or resource busy",
        EXIST => "File exists",
        XDEV => "Cross-device link",
        NODEV => "No such device",
        NOTDIR => "Not a directory",
        ISDIR => "Is a directory",
        INVAL => "Invalid argument",
        NFILE => "File table overflow",
        MFILE => "Too many open files",
        NOTTY => "Not a typewriter",
        TXTBSY => "Text file busy",
        FBIG => "File too large",
        NOSPC => "No space left on device",
        SPIPE => "Illegal seek",
        ROFS => "Read-only file system",
        MLINK => "Too many links",
        PIPE => "Broken pipe",
        DOM => "Math argument out of domain of func",
        RANGE => "Math result not representable",
        DEADLK => "Resource deadlock would occur",
        NAMETOOLONG => "File name too long",
        NOLCK => "No record locks available",
        NOSYS => "Function not implemented",
        NOTEMPTY => "Directory not empty",
        LOOP => "Too many symbolic links encountered",
        NOMSG => "No message of desired type",
        IDRM => "Identifier removed",
        INPROGRESS => "Operation now in progress",
        ALREADY => "Operation already in progress",
        NOTSOCK => "Socket operation on non-socket",
        DESTADDRREQ => "Destination address required",
        MSGSIZE => "Message too long",
        PROTOTYPE => "Protocol wrong type for socket",
        NOPROTOOPT => "Protocol not available",
        PROTONOSUPPORT => "Protocol not supported",
        SOCKTNOSUPPORT => "Socket type not supported",
        PFNOSUPPORT => "Protocol family not supported",
        AFNOSUPPORT => "Address family not supported by protocol",
        ADDRINUSE => "Address already in use",
        ADDRNOTAVAIL => "Cannot assign requested address",
        NETDOWN => "Network is down",
        NETUNREACH => "Network is unreachable",
        NETRESET => "Network dropped connection because of reset",
        CONNABORTED => "Software caused connection abort",
        CONNRESET => "Connection reset by peer",
        NOBUFS => "No buffer space available",
        ISCONN => "Transport endpoint is already connected",
        NOTCONN => "Transport endpoint is not connected",
        SHUTDOWN => "Cannot send after transport endpoint shutdown",
        TOOMANYREFS => "Too many references: cannot splice",
        TIMEDOUT => "Connection timed out",
        CONNREFUSED => "Connection refused",
        HOSTDOWN => "Host is down",
        HOSTUNREACH => "No route to host",
        CHRNG => "Channel number out of range",
        L2NSYNC => "Level 2 not synchronized",
        L3HLT => "Level 3 halted",
        L3RST => "Level 3 reset",
        LNRNG => "Link number out of range",
        UNATCH => "Protocol driver not attached",
        NOCSI => "No CSI structure available",
        L2HLT => "Level 2 halted",
        BADE => "Invalid exchange",
        BADR => "Invalid request descriptor",
        XFULL => "Exchange full",
        NOANO => "No anode",
        BADRQC => "Invalid request code",
        BADSLT => "Invalid slot",
        BFONT => "Bad font file format",
        NOSTR => "Device not a stream",
        NODATA => "No data available",
        TIME => "Timer expired",
        NOSR => "Out of streams resources",
        NONET => "Machine is not on the network",
        NOPKG => "Package not installed",
        REMOTE => "Object is remote",
        NOLINK => "Link has been severed",
        ADV => "Advertise error",
        SRMNT => "Srmount error",
        COMM => "Communication error on send",
        PROTO => "Protocol error",
        MULTIHOP => "Multihop attempted",
        DOTDOT => "RFS specific error",
        BADMSG => "Not a data message",
        OVERFLOW => "Value too large for defined data type",
        NOTUNIQ => "Name not unique on network",
        BADFD => "File descriptor in bad state",
        REMCHG => "Remote address changed",
        LIBACC => "Can not access a needed shared library",
        LIBBAD => "Accessing a corrupted shared library",
        LIBSCN => ".lib section in a.out corrupted",
        LIBMAX => "Attempting to link in too many shared libraries",
        LIBEXEC => "Cannot exec a shared library directly",
        ILSEQ => "Illegal byte sequence",
        RESTART => "Interrupted system call should be restarted",
        STRPIPE => "Streams pipe error",
        USERS => "Too many users",
        OPNOTSUPP => "Operation not supported on transport endpoint",
        STALE => "Stale file handle",
        UCLEAN => "Structure needs cleaning",
        NOTNAM => "Not a XENIX named type file",
        NAVAIL => "No XENIX semaphores available",
        ISNAM => "Is a named type file",
        REMOTEIO => "Remote I/O error",
        DQUOT => "Quota exceeded",
        NOMEDIUM => "No medium found",
        MEDIUMTYPE => "Wrong medium type",
        CANCELED => "Operation canceled",
        NOKEY => "Required key not available",
        KEYEXPIRED => "Key has expired",
        KEYREVOKED => "Key has been revoked",
        KEYREJECTED => "Key was rejected by service",
        OWNERDEAD => "Owner died",
        NOTRECOVERABLE => "State not recoverable",
        RFKILL => "Operation not possible due to RF-kill",
        HWPOISON => "Memory page has hardware error",
    }
}
