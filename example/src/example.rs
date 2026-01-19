//! # Example Program

extern crate abi;



#[unsafe(no_mangle)]
pub extern "C" fn render(bounds: abi::Aabb2D<f32>) -> abi::RenderPass {
    abi::RenderPass {
        bounds,
        layers: vec![abi::RenderLayer {
            objects: vec![
                abi::RenderObject::Quad {
                    bounds,
                    color: abi::Rgba {
                        r: 0xff,
                        g: 0x11,
                        b: 0x11,
                        a: 0xff,
                    },
                },
                abi::RenderObject::Quad {
                    bounds: abi::Aabb2D {
                        x_max: bounds.x_min + 50.0,
                        y_max: bounds.y_min + 50.0,
                        ..bounds
                    },
                    color: abi::Rgba {
                        r: 0x11,
                        g: 0xff,
                        b: 0x11,
                        a: 0xff,
                    },
                },
            ]
            .into(),
        }]
        .into(),
    }
}
