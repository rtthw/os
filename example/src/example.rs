//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;

use abi::*;



manifest! {
    name: "example",
    init: || {
        ElementBuilder::new(Column::new()
            .with(Label::new("One"))
            .with(Label::new("Two")))
    },
    dependencies: &[],
}
