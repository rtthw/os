//! # Cursor Management

use std::io::Read as _;

use {abi::CursorIcon, anyhow::Result, log::warn, xcursor::parser::Image};



pub struct CursorData {
    icons: Vec<Image>,
}

static FALLBACK_CURSOR_DATA: &[u8] = include_bytes!("../../../res/cursor.rgba");

impl CursorData {
    pub fn load_or_fallback(path: &str) -> Self {
        match Self::load(path) {
            Ok(data) => data,
            Err(error) => {
                warn!("Failed to load cursor data at '{path}': {error}");
                Self {
                    icons: vec![Image {
                        size: 32,
                        width: 64,
                        height: 64,
                        xhot: 1,
                        yhot: 1,
                        delay: 1,
                        pixels_rgba: Vec::from(FALLBACK_CURSOR_DATA),
                        pixels_argb: vec![], // Unused.
                    }],
                }
            }
        }
    }

    pub fn load(path: &str) -> Result<Self> {
        let mut file = std::fs::File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let icons = xcursor::parser::parse_xcursor(&data)
            .ok_or(anyhow::anyhow!("Failed to parse XCursor at '{path}'"))?;

        Ok(Self { icons })
    }

    pub fn get_image(&self, scale: u32, millis: u32) -> Image {
        let size = 24 * scale;
        frame(millis, size, &self.icons)
    }
}

fn nearest_images(size: u32, images: &[Image]) -> impl Iterator<Item = &Image> {
    // Follow the nominal size of the cursor to choose the nearest.
    let nearest_image = images
        .iter()
        .min_by_key(|image| u32::abs_diff(size, image.size))
        .unwrap();

    images.iter().filter(move |image| {
        image.width == nearest_image.width && image.height == nearest_image.height
    })
}

fn frame(mut millis: u32, size: u32, images: &[Image]) -> Image {
    let total = nearest_images(size, images).fold(0, |acc, image| acc + image.delay);

    if total == 0 {
        millis = 0;
    } else {
        millis %= total;
    }

    for img in nearest_images(size, images) {
        if millis <= img.delay {
            return img.clone();
        }
        millis -= img.delay;
    }

    unreachable!()
}



pub fn egui_to_abi_cursor_icon(value: egui::CursorIcon) -> CursorIcon {
    match value {
        egui::CursorIcon::AllScroll => CursorIcon::AllScroll,
        egui::CursorIcon::Grab => CursorIcon::Grab,
        egui::CursorIcon::Grabbing => CursorIcon::Grabbing,
        egui::CursorIcon::Help => CursorIcon::Help,
        egui::CursorIcon::NoDrop => CursorIcon::NoDrop,
        egui::CursorIcon::PointingHand => CursorIcon::PointingHand,
        egui::CursorIcon::ResizeColumn => CursorIcon::SplitH,
        egui::CursorIcon::ResizeHorizontal => CursorIcon::SplitH,
        egui::CursorIcon::ResizeRow => CursorIcon::SplitV,
        egui::CursorIcon::ResizeVertical => CursorIcon::SplitV,
        egui::CursorIcon::Text => CursorIcon::IBeam,
        egui::CursorIcon::ZoomIn => CursorIcon::ZoomIn,
        egui::CursorIcon::ZoomOut => CursorIcon::ZoomOut,
        _ => CursorIcon::Default,
    }
}
