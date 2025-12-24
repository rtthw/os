
use crate::{c_str::{AsCStr, InvalidCStr, map_cstr_opt}, raw};



pub fn mount<
    S1: AsCStr + ?Sized,
    S2: AsCStr + ?Sized,
    S3: AsCStr + ?Sized,
    S4: AsCStr + ?Sized,
>(
    source: &S1,
    target: &S2,
    fs: &S3,
    flags: u64,
    options: Option<&S4>,
) -> Result<(), MountError> {
    let errno = source.map_cstr(|source| {
        target.map_cstr(|target| {
            fs.map_cstr(|fs| {
                map_cstr_opt(options, |options| {
                    raw::mount(
                        source,
                        target,
                        fs,
                        flags,
                        options,
                    )
                })
            })
        })
    })????;

    if errno < 0 {
        todo!()
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub enum MountError {
    InvalidInput,
}

impl From<InvalidCStr> for MountError {
    fn from(_value: InvalidCStr) -> Self {
        Self::InvalidInput
    }
}
