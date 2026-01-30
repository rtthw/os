//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;

use abi::*;



manifest! {
    name: "example",
    init: || {
        Example { font_size: 10.0 }.wrap()
    },
    dependencies: &[],
}

struct Example {
    font_size: f32,
}

impl App<Update> for Example {
    fn view(&mut self) -> impl Element + 'static {
        Column::new()
            .with(Label::new("One"))
            .with(Label::new("Two"))
    }

    fn update(&mut self, update: Update) -> Result<(), &'static str> {
        match update {
            Update::IncreaseFontSize(value) => {
                if value < 0.0 {
                    return Err("cannot decrease font size");
                }
                self.font_size += value;
            }
        }

        Ok(())
    }
}

enum Update {
    IncreaseFontSize(f32),
}
