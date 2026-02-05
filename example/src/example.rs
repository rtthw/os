//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;

use abi::*;



manifest! {
    name: "example",
    init: || {
        ElementBuilder::new(Column::new()
            .with(Label::new("Zero"))
            .with(Row::new()
                .with(Label::new("One").with_font_size(12.0))
                .with(Label::new("Two").with_font_size(24.0))
                .with(Label::new("Three").with_font_size(36.0)))
            .with(Label::new("Four"))
            .with(Label::new("Five")))
    },
    dependencies: &[],
}
