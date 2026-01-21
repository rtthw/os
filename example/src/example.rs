//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;

use abi::AsClickable as _;



abi::manifest! {
    name: "example",
    init: || {
        use abi::App as _;
        App { field: 2.0 }.wrap()
    },
    dependencies: &[],
}

struct App {
    field: f32,
}

impl abi::App<Update> for App {
    fn view(&mut self, _bounds: abi::Aabb2D<f32>) -> impl abi::View<Update> {
        abi::Label::new("Click Me").on_click(|| Update::ChangeField(7.0))
    }

    fn update(&mut self, update: Update) -> Result<(), &'static str> {
        match update {
            Update::ChangeField(value) => {
                if value == 0.0 {
                    return Err("cannot divide by zero");
                }
                self.field = value;
            }
        }

        Ok(())
    }
}

enum Update {
    ChangeField(f32),
}
