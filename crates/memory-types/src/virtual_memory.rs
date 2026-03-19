//! # Virtual Memory

use core::{
    fmt,
    ops::{Add, AddAssign, Deref, Sub, SubAssign},
};

use crate::{PAGE_SIZE, align_down, align_up};

pub const MAX_VIRTUAL_ADDR: usize = usize::MAX; // 0xFFFF_FFFF_FFFF_FFFF
pub const VIRTUAL_MEMORY_SHIFT: usize = 47;
pub const VIRTUAL_MEMORY_OFFSET: usize = MAX_VIRTUAL_ADDR << VIRTUAL_MEMORY_SHIFT;


/// A virtual memory page.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Page {
    number: usize,
}

impl Page {
    #[inline]
    pub const fn new(number: usize) -> Self {
        Self { number }
    }

    #[inline]
    pub const fn containing_addr(addr: VirtualAddress) -> Self {
        Self {
            number: addr.to_raw() / PAGE_SIZE,
        }
    }

    #[inline]
    pub const fn number(self) -> usize {
        self.number
    }

    #[inline]
    pub const fn base_addr(self) -> VirtualAddress {
        unsafe { VirtualAddress::new_unchecked(self.number * PAGE_SIZE) }
    }
}

impl fmt::Debug for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Page #{} @ {:#x}", self.number(), self.base_addr())
    }
}

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}



/// An address in virtual memory.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VirtualAddress(usize);

impl VirtualAddress {
    #[inline]
    pub const fn new(addr: usize) -> Self {
        // Sign-extend the value by doing a right shift on it as an isize.
        Self(((addr << 16) as isize >> 16) as usize)
    }

    #[inline]
    pub const unsafe fn new_unchecked(addr: usize) -> Self {
        Self(addr)
    }

    #[inline]
    pub const fn to_raw(self) -> usize {
        self.0
    }

    #[inline]
    pub const fn is_page_aligned(self) -> bool {
        self.0 & (PAGE_SIZE - 1) == 0
    }

    #[inline]
    pub const fn page_align_down(self) -> Self {
        Self(align_down(self.0, PAGE_SIZE))
    }

    #[inline]
    pub const fn page_align_up(self) -> Self {
        Self(align_up(self.0, PAGE_SIZE))
    }

    #[inline]
    pub const fn page(self) -> Page {
        Page::containing_addr(self)
    }
}

impl Deref for VirtualAddress {
    type Target = usize;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<usize> for VirtualAddress {
    #[inline]
    fn from(value: usize) -> Self {
        Self::new(value)
    }
}

impl fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x} (VIR)", self.0)
    }
}

impl fmt::Display for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl fmt::Binary for VirtualAddress {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Binary::fmt(&self.0, f)
    }
}

impl fmt::Octal for VirtualAddress {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Octal::fmt(&self.0, f)
    }
}

impl fmt::LowerHex for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

impl Add<usize> for VirtualAddress {
    type Output = Self;

    #[inline]
    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.0.checked_add(rhs).unwrap())
    }
}

impl AddAssign<usize> for VirtualAddress {
    #[inline]
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl Sub<usize> for VirtualAddress {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: usize) -> Self::Output {
        Self::new(self.0.checked_sub(rhs).unwrap())
    }
}

impl SubAssign<usize> for VirtualAddress {
    #[inline]
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs;
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        let addr = VirtualAddress::new(PAGE_SIZE);
        assert!(addr.is_page_aligned());
        assert_eq!(*addr, PAGE_SIZE);

        let addr = VirtualAddress::new(0x3777);
        assert!(!addr.is_page_aligned());
        assert_eq!(*addr.page_align_down(), 0x3000);
        assert_eq!(*addr.page_align_up(), 0x4000);

        let page = Page::new(7);
        assert_eq!(page.number(), 7);
        assert_eq!(*page.base_addr(), 7 * PAGE_SIZE);

        let page = Page::containing_addr(VirtualAddress::new(0x300111));
        assert_eq!(page.number(), 0x300);
    }

    #[test]
    fn addr_truncates() {
        let addr = VirtualAddress::new(0);
        assert_eq!(*addr, 0);
        let addr = VirtualAddress::new(1 << VIRTUAL_MEMORY_SHIFT);
        assert_eq!(*addr, VIRTUAL_MEMORY_OFFSET);
        let addr = VirtualAddress::new(43);
        assert_eq!(*addr, 43);
        let addr = VirtualAddress::new(5555 << VIRTUAL_MEMORY_SHIFT);
        assert_eq!(*addr, VIRTUAL_MEMORY_OFFSET);
    }


    #[test]
    #[should_panic]
    fn addr_overflow() {
        _ = VirtualAddress::new(MAX_VIRTUAL_ADDR) + 1;
    }

    #[test]
    #[should_panic]
    fn addr_underflow() {
        _ = VirtualAddress::new(0) - 1;
    }
}
