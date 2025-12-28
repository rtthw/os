


/// Generic error type.
#[repr(i32)]
pub enum Error {
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
    /// Multihop attempted.
    MULTIHOP = 72,
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
