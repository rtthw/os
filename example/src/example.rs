//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;

use abi::{AsCentered as _, AsClickable as _};



abi::manifest! {
    name: "example",
    init: || {
        use abi::App as _;
        App { font_size: 2.0 }.wrap()
    },
    dependencies: &[],
}

struct App {
    font_size: f32,
}

impl abi::App<Update> for App {
    fn view(&mut self, _bounds: abi::Aabb2D<f32>) -> impl abi::View<Update> {
        abi::Label::new("Click Me")
            .font_size(self.font_size)
            .on_click(|| Update::IncreaseFontSize(2.0))
            .centered()
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
