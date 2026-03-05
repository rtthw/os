use crate::{
    Error, Result,
    c_str::{AsCStr, map_cstr_opt},
    constants, raw,
};



pub fn mount<S1: AsCStr + ?Sized, S2: AsCStr + ?Sized, S3: AsCStr + ?Sized, S4: AsCStr + ?Sized>(
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
                    raw::mount(source, target, fs, flags.0, options)
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



pub const BIND: MountFlags = MountFlags(constants::MS_BIND);
pub const DIRSYNC: MountFlags = MountFlags(constants::MS_DIRSYNC);
pub const LAZYTIME: MountFlags = MountFlags(constants::MS_LAZYTIME);
pub const MANDLOCK: MountFlags = MountFlags(constants::MS_MANDLOCK);
pub const MOVE: MountFlags = MountFlags(constants::MS_MOVE);
pub const NOATIME: MountFlags = MountFlags(constants::MS_NOATIME);
pub const NODEV: MountFlags = MountFlags(constants::MS_NODEV);
pub const NODIRATIME: MountFlags = MountFlags(constants::MS_NODIRATIME);
pub const NOEXEC: MountFlags = MountFlags(constants::MS_NOEXEC);
pub const NOSUID: MountFlags = MountFlags(constants::MS_NOSUID);
pub const NOSYMFOLLOW: MountFlags = MountFlags(constants::MS_NOSYMFOLLOW);
pub const POSIXACL: MountFlags = MountFlags(constants::MS_POSIXACL);
pub const PRIVATE: MountFlags = MountFlags(constants::MS_PRIVATE);
pub const RDONLY: MountFlags = MountFlags(constants::MS_RDONLY);
pub const REC: MountFlags = MountFlags(constants::MS_REC);
pub const RELATIME: MountFlags = MountFlags(constants::MS_RELATIME);
pub const REMOUNT: MountFlags = MountFlags(constants::MS_REMOUNT);
pub const SHARED: MountFlags = MountFlags(constants::MS_SHARED);
pub const SILENT: MountFlags = MountFlags(constants::MS_SILENT);
pub const SLAVE: MountFlags = MountFlags(constants::MS_SLAVE);
pub const STRICTATIME: MountFlags = MountFlags(constants::MS_STRICTATIME);
pub const SYNCHRONOUS: MountFlags = MountFlags(constants::MS_SYNCHRONOUS);
pub const UNBINDABLE: MountFlags = MountFlags(constants::MS_UNBINDABLE);

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct MountFlags(u64);

impl core::ops::BitOr for MountFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
