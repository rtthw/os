
use crate::{Error, Result, c_str::{AsCStr, map_cstr_opt}, raw};



pub fn mount<
    S1: AsCStr + ?Sized,
    S2: AsCStr + ?Sized,
    S3: AsCStr + ?Sized,
    S4: AsCStr + ?Sized,
>(
    source: &S1,
    target: &S2,
    fs: &S3,
    flags: MountFlags,
    options: Option<&S4>,
) -> Result<()> {
    let errno = source.map_cstr(|source| {
        target.map_cstr(|target| {
            fs.map_cstr(|fs| {
                map_cstr_opt(options, |options| {
                    raw::mount(
                        source,
                        target,
                        fs,
                        flags.0,
                        options,
                    )
                })
            })
        })
    })????;

    if errno < 0 {
        Err(Error::latest())
    } else {
        Ok(())
    }
}



pub const BIND: MountFlags = MountFlags(libc::MS_BIND);
pub const DIRSYNC: MountFlags = MountFlags(libc::MS_DIRSYNC);
pub const LAZYTIME: MountFlags = MountFlags(libc::MS_LAZYTIME);
pub const MANDLOCK: MountFlags = MountFlags(libc::MS_MANDLOCK);
pub const MOVE: MountFlags = MountFlags(libc::MS_MOVE);
pub const NOATIME: MountFlags = MountFlags(libc::MS_NOATIME);
pub const NODEV: MountFlags = MountFlags(libc::MS_NODEV);
pub const NODIRATIME: MountFlags = MountFlags(libc::MS_NODIRATIME);
pub const NOEXEC: MountFlags = MountFlags(libc::MS_NOEXEC);
pub const NOSUID: MountFlags = MountFlags(libc::MS_NOSUID);
pub const NOSYMFOLLOW: MountFlags = MountFlags(libc::MS_NOSYMFOLLOW);
pub const POSIXACL: MountFlags = MountFlags(libc::MS_POSIXACL);
pub const PRIVATE: MountFlags = MountFlags(libc::MS_PRIVATE);
pub const RDONLY: MountFlags = MountFlags(libc::MS_RDONLY);
pub const REC: MountFlags = MountFlags(libc::MS_REC);
pub const RELATIME: MountFlags = MountFlags(libc::MS_RELATIME);
pub const REMOUNT: MountFlags = MountFlags(libc::MS_REMOUNT);
pub const SHARED: MountFlags = MountFlags(libc::MS_SHARED);
pub const SILENT: MountFlags = MountFlags(libc::MS_SILENT);
pub const SLAVE: MountFlags = MountFlags(libc::MS_SLAVE);
pub const STRICTATIME: MountFlags = MountFlags(libc::MS_STRICTATIME);
pub const SYNCHRONOUS: MountFlags = MountFlags(libc::MS_SYNCHRONOUS);
pub const UNBINDABLE: MountFlags = MountFlags(libc::MS_UNBINDABLE);

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct MountFlags(u64);

impl core::ops::BitOr for MountFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
