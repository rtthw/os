//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;

use abi::*;



manifest! {
    name: "example",
    init: || {
        ElementBuilder::new(Column::new()
            .with(Row::new()))
    },
    dependencies: &[],
}
