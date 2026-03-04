//! # Plain Old Data (POD)

#![no_std]

/// Marker trait for plain old data types.
pub unsafe trait Pod: Sized {}

unsafe impl Pod for u8 {}
unsafe impl Pod for u16 {}
unsafe impl Pod for u32 {}
unsafe impl Pod for u64 {}
unsafe impl Pod for u128 {}
unsafe impl Pod for usize {}

unsafe impl Pod for i8 {}
unsafe impl Pod for i16 {}
unsafe impl Pod for i32 {}
unsafe impl Pod for i64 {}
unsafe impl Pod for i128 {}
unsafe impl Pod for isize {}

pub unsafe fn read<T: Pod>(bytes: &[u8]) -> &T {
    assert!(size_of::<T>() <= bytes.len());
    let addr = bytes.as_ptr() as usize;
    // Alignment is always a power of 2, so we can use bit ops instead of a mod
    // here.
    assert!((addr & (align_of::<T>() - 1)) == 0);

    unsafe { &*(bytes.as_ptr() as *const T) }
}

pub fn read_until_null(input: &[u8]) -> &[u8] {
    for (i, byte) in input.iter().enumerate() {
        if *byte == 0 {
            return &input[..i];
        }
    }

    panic!("no null byte found in input");
}

pub fn read_str(input: &[u8]) -> &str {
    core::str::from_utf8(read_str_bytes(input)).expect("invalid UTF-8 string")
}

pub fn read_str_bytes(input: &[u8]) -> &[u8] {
    for (i, byte) in input.iter().enumerate() {
        if *byte == 0 {
            return &input[..i];
        }
    }

    panic!("no null byte in input");
}

pub fn read_array<T: Pod>(input: &[u8]) -> &[T] {
    let element_size = size_of::<T>();
    assert!(element_size > 0, "can't read arrays of zero-sized types");
    assert!(input.len() % element_size == 0);
    let addr = input.as_ptr() as usize;
    assert!(addr & (align_of::<T>() - 1) == 0);

    unsafe { read_array_unsafe(input) }
}

pub unsafe fn read_array_unsafe<T: Sized>(input: &[u8]) -> &[T] {
    let ptr = input.as_ptr() as *const T;
    unsafe { core::slice::from_raw_parts(ptr, input.len() / size_of::<T>()) }
}
