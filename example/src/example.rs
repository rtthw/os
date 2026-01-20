//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;



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
    fn render(&mut self, bounds: abi::Aabb2D<f32>) -> abi::RenderPass<'_> {
        let width = bounds.x_max - bounds.x_min;
        let height = bounds.y_max - bounds.y_min;

        let center_x = width / self.field;
        let center_y = height / self.field;

        abi::RenderPass {
            bounds: bounds,
            layers: vec![abi::RenderLayer {
                objects: vec![
                    abi::RenderObject::Quad {
                        bounds: bounds,
                        color: abi::Rgba {
                            r: 0x11,
                            g: 0x11,
                            b: 0x11,
                            a: 0xff,
                        },
                    },
                    abi::RenderObject::Quad {
                        bounds: abi::Aabb2D {
                            x_min: bounds.x_min + center_x - 40.0,
                            x_max: bounds.x_min + center_x + 40.0,
                            y_min: bounds.y_min + center_y - 12.0,
                            y_max: bounds.y_min + center_y + 14.0,
                        },
                        color: abi::Rgba {
                            r: 0xd9,
                            g: 0x6d,
                            b: 0x81,
                            a: 0xff,
                        },
                    },
                    abi::RenderObject::Text {
                        text: "Example".into(),
                        bounds: abi::Aabb2D {
                            x_min: bounds.x_min + center_x,
                            y_min: bounds.y_min + center_y,
                            ..bounds
                        },
                        color: abi::Rgba {
                            r: 0x1e,
                            g: 0x1e,
                            b: 0x22,
                            a: 0xff,
                        },
                        font_size: 20.0,
                    },
                    abi::RenderObject::Button {
                        text: "Click Me".into(),
                        on_click: || Box::new(Update::ChangeField(5.0)),
                    },
                ]
                .into(),
            }]
            .into(),
        }
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
