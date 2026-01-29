//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;

use abi::Element as _;



abi::manifest! {
    name: "example",
    init: || {
        use abi::App as _;
        println!(
            "example::Label::children_ids = {:?}",
            abi::Label::new("Example").children_ids(),
        );
        App { font_size: 10.0 }.wrap()
    },
    dependencies: &[],
}

struct App {
    font_size: f32,
}

impl abi::App<Update> for App {
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
