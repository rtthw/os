//! # Cursor Icon



#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CursorIcon {
    #[default]
    Default,

    AllScroll,
    Grab,
    Grabbing,
    Help,
    IBeam,
    NoDrop,
    PointingHand,
    SplitH,
    SplitV,
    ZoomIn,
    ZoomOut,
}

impl CursorIcon {
    pub const fn name(&self) -> &'static str {
        match self {
            CursorIcon::Default => "default",
            CursorIcon::AllScroll => "all_scroll",
            CursorIcon::Grab => "grab",
            CursorIcon::Grabbing => "grabbing",
            CursorIcon::Help => "help",
            CursorIcon::IBeam => "ibeam",
            CursorIcon::NoDrop => "no_drop",
            CursorIcon::PointingHand => "pointing_hand",
            CursorIcon::SplitH => "split_h",
            CursorIcon::SplitV => "split_v",
            CursorIcon::ZoomIn => "zoom_in",
            CursorIcon::ZoomOut => "zoom_out",
        }
    }
}
