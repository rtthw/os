//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;

use abi::*;



manifest! {
    name: "example",
    init: || {
        ElementBuilder::new(VerticalScroll::new(abi::column![
            gap: 50.0;
            Label::new("Zero").with_color(Rgba::rgb(0xaa, 0xaa, 0xad)),
            LineInput::new("Type something...").with_font_size(20.0),
            abi::row![
                gap: 10.0;
                Label::new("One")
                    .with_font_size(12.0)
                    .with_color(Rgba::rgb(0x73, 0x73, 0x89)),
                Label::new("Two").with_font_size(24.0),
                abi::column![
                    gap: 10.0;
                    Label::new("Four"),
                    Label::new("Five").with_font_size(48.0),
                    Label::new("Six").with_font_size(36.0),
                ],
                Label::new("Three").with_font_size(36.0),
                Label::new("Three").with_font_size(24.0),
                Label::new("Three").with_font_size(12.0),
                abi::column![
                    gap: 10.0;
                    Label::new("Four"),
                    Label::new("Five").with_font_size(48.0),
                ],
                Label::new("Six"),
            ],
            Label::new("Seven"),
            Label::new("Eight"),
            Label::new("Nine"),
            Label::new("Ten"),
            Label::new("Eleven"),
            Label::new("Twelve"),
        ]))
    },
    dependencies: &[],
}
