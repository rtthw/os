//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;

use abi::*;



manifest! {
    name: "example",
    init: || {
        ElementBuilder::new(VerticalScroll::new(abi::column![
            Flex::row()
                .with_cross_align(CrossAlignment::Center)
                .with(Label::new("Zero")
                    .with_color(Rgba::rgb(0xaa, 0xaa, 0xad))
                    .with_font_size(10.0), 1.0)
                .with(Label::new("One").with_font_size(15.0), 1.0)
                .with(Flex::column()
                    .with_cross_align(CrossAlignment::Start)
                    .with(LineInput::new("Type something...").with_font_size(20.0), 0.0)
                    .with(Label::new("Two").with_font_size(20.0), 0.0)
                    .with(Label::new("Three").with_font_size(25.0), 0.0)
                    .with_spacer(1.0)
                    .with(Label::new("Four").with_font_size(30.0), 1.0)
                    .with(Label::new("Five").with_font_size(35.0), 1.0)
                    .with(Label::new("Six").with_font_size(40.0), 2.0), 1.0)
                .with(Label::new("Seven").with_font_size(45.0), 2.0),
            Label::new("Eight").with_font_size(50.0),
        ]))
    },
    dependencies: &[],
}
