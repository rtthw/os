//! # Framebuffer Management

#![no_std]

use boot_info::{DisplayInfo, PixelFormat};



pub struct Framebuffer {
    ptr: *mut u32,
    width: usize,
    height: usize,
    format: PixelFormat,
}

impl Framebuffer {
    pub fn from_display_info(display_info: &DisplayInfo) -> Self {
        assert_eq!(
            display_info.framebuffer_size / 4,
            (display_info.stride * display_info.height) as usize,
        );

        Self {
            ptr: display_info.framebuffer_addr as *mut u32,
            width: display_info.stride as usize,
            height: display_info.height as usize,
            format: display_info.format,
        }
    }

    pub fn clear_screen(&mut self, color: Color) {
        let color = color.to_u32(self.format);

        unsafe {
            core::slice::from_raw_parts_mut(self.ptr, self.width * self.height).fill(color);
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        let color = color.to_u32(self.format);

        let x = x.clamp(0, self.width as _) as usize;
        let y = y.clamp(0, self.height as _) as usize;
        let w = w.clamp(0, self.width.saturating_sub(x) as _) as usize;
        let h = h.clamp(0, self.height.saturating_sub(y) as _) as usize;

        if w == 0 || h == 0 {
            return;
        }

        unsafe {
            let mut ptr = self.ptr.add(y * self.width + x);
            core::slice::from_raw_parts_mut(ptr, w).fill(color);
            for _ in 1..h {
                let src = ptr;
                ptr = ptr.add(self.width);
                src.copy_to_nonoverlapping(ptr, w);
            }
        }
    }
}



#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);

    #[inline]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn to_u32(self, format: PixelFormat) -> u32 {
        match format {
            PixelFormat::Bgr => (self.r as u32) << 16 | (self.g as u32) << 8 | (self.b as u32) << 0,
            PixelFormat::Rgb => (self.r as u32) << 0 | (self.g as u32) << 8 | (self.b as u32) << 16,
        }
    }
}
