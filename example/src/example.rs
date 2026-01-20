//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;



abi::manifest! {
    name: "example",
    render: render,
    dependencies: &[],
}

pub extern "C" fn render<'a>(bounds: &'a abi::Aabb2D<f32>) -> abi::RenderPass<'a> {
    let width = bounds.x_max - bounds.x_min;
    let height = bounds.y_max - bounds.y_min;

    let center_x = width / 2.0;
    let center_y = height / 2.0;

    abi::RenderPass {
        bounds: *bounds,
        layers: vec![abi::RenderLayer {
            objects: vec![
                abi::RenderObject::Quad {
                    bounds: *bounds,
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
                        ..*bounds
                    },
                    color: abi::Rgba {
                        r: 0x1e,
                        g: 0x1e,
                        b: 0x22,
                        a: 0xff,
                    },
                    font_size: 20.0,
                },
            ]
            .into(),
        }]
        .into(),
    }
}
