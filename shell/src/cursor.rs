//! # Cursor Management

use std::io::Read as _;

use {anyhow::Result, log::warn, xcursor::parser::Image};



#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CursorIcon {
    #[default]
    Default,
    PointingHand,
    SplitH,
    SplitV,
}

impl CursorIcon {
    pub const fn name(&self) -> &'static str {
        match self {
            CursorIcon::Default => "default",
            CursorIcon::PointingHand => "pointing_hand",
            CursorIcon::SplitH => "split_h",
            CursorIcon::SplitV => "split_v",
        }
    }
}

pub struct CursorData {
    icons: Vec<Image>,
}

static FALLBACK_CURSOR_DATA: &[u8] = include_bytes!("../../res/cursor.rgba");

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
            .ok_or(anyhow::anyhow!("Failed to parse XCurosr at '{path}'"))?;

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



impl From<egui::CursorIcon> for CursorIcon {
    fn from(value: egui::CursorIcon) -> Self {
        match value {
            egui::CursorIcon::PointingHand => Self::PointingHand,
            egui::CursorIcon::ResizeColumn => Self::SplitH,
            egui::CursorIcon::ResizeHorizontal => Self::SplitH,
            egui::CursorIcon::ResizeRow => Self::SplitV,
            egui::CursorIcon::ResizeVertical => Self::SplitV,
            _ => Self::Default,
        }
    }
}
