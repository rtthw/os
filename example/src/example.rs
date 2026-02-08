//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;

use abi::*;



manifest! {
    name: "example",
    init: || {
        ElementBuilder::new(VerticalScroll::new(Column::new()
            .with_gap(50.0)
            .with(Label::new("Zero"))
            .with(Row::new()
                .with_gap(10.0)
                .with(Label::new("One").with_font_size(12.0))
                .with(Label::new("Two").with_font_size(24.0))
                .with(Column::new()
                    .with_gap(10.0)
                    .with(Label::new("Four"))
                    .with(Label::new("Five").with_font_size(48.0))
                    .with(Label::new("Six").with_font_size(36.0)))
                .with(Label::new("Three").with_font_size(36.0))
                .with(Label::new("Three").with_font_size(24.0))
                .with(Label::new("Three").with_font_size(12.0))
                .with(Column::new()
                    .with_gap(10.0)
                    .with(Label::new("Four"))
                    .with(Label::new("Five").with_font_size(48.0)))
                    .with(Label::new("Six")))
            .with(Label::new("Seven"))
            .with(Label::new("Eight"))
            .with(Label::new("Nine"))
            .with(Label::new("Ten"))
            .with(Label::new("Eleven"))
            .with(Label::new("Twelve"))))
    },
    dependencies: &[],
}
