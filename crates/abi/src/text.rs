//! # Text Types



#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum TextWrapMode {
    #[default]
    Wrap = 0,
    NoWrap = 1,
}

/// How text content is aligned within a container.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum TextAlignment {
    /// Align text content to the beginning edge.
    ///
    /// This is equivalent to [`TextAlignment::Left`] for left-to-right text,
    /// and [`TextAlignment::Right`] for right-to-left text.
    #[default]
    Start = 0,
    /// Align text content to the ending edge.
    ///
    /// This is equivalent to [`TextAlignment::Right`] for left-to-right text,
    /// and [`TextAlignment::Left`] for right-to-left text.
    End = 1,
    /// Align text content to the left edge.
    Left = 2,
    /// Align text content to the center.
    Center = 3,
    /// Align text content to the right edge.
    Right = 4,
    /// Justify text content to fill all available space, with the last line
    /// unaffected.
    Justify = 5,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    Relative(f32),
    Absolute(f32),
}

impl Default for LineHeight {
    fn default() -> Self {
        Self::FONT_PREFERRED
    }
}

impl LineHeight {
    pub const FONT_PREFERRED: Self = Self::Relative(1.0);
}
