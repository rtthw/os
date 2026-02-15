//! # Example Program

#![forbid(unsafe_code)]

extern crate abi;

use abi::*;



manifest! {
    name: "example",
    init: || {
        ElementBuilder::new(VerticalScroll::new(abi::column![
            Flex::row()
                .with_cross_align(CrossAlignment::Center)
                .with(Label::new("Zero")
                    .with_color(Rgba::rgb(0xaa, 0xaa, 0xad))
                    .with_font_size(10.0), 1.0)
                .with(Label::new("One").with_font_size(15.0), 1.0)
                .with(Flex::column()
                    .with_cross_align(CrossAlignment::Start)
                    .with(LineInput::new("Type something...").with_font_size(20.0), 0.0)
                    .with(Label::new("Two").with_font_size(20.0), 0.0)
                    .with(Label::new("Three").with_font_size(25.0), 0.0)
                    .with(Flex::row()
                        .with_main_align(AxisAlignment::SpaceAround)
                        .with(Label::new("A"), 0.0)
                        .with(Label::new("B"), 0.0)
                        .with(Label::new("C"), 0.0), 0.0)
                    .with(Flex::row()
                        .with_main_align(AxisAlignment::SpaceBetween)
                        .with(button("A"), 0.0)
                        .with(button("B"), 0.0)
                        .with(button("C"), 0.0), 0.0)
                    .with(Flex::row()
                        .with_main_align(AxisAlignment::SpaceEvenly)
                        .with(Label::new("A"), 0.0)
                        .with(Label::new("B"), 0.0)
                        .with(Label::new("C"), 0.0), 0.0)
                    .with_spacer(1.0)
                    .with(Label::new("Four").with_font_size(30.0), 1.0)
                    .with(Label::new("Five").with_font_size(35.0), 1.0)
                    .with(Label::new("Six").with_font_size(40.0), 2.0), 1.0)
                .with(Label::new("Seven").with_font_size(45.0), 2.0),
            Label::new("Eight").with_font_size(50.0),
        ]))
    },
    dependencies: &[],
}



fn button(text: &str) -> OnHover<OnClick<Label>> {
    OnHover::new(
        OnClick::new(
            Label::new(text)
                .with_font_size(16.0)
                .with_color(Rgba::rgb(0xaa, 0xaa, 0xad)),
            |label, pass, mouse_down| {
                if mouse_down {
                    label.font_size = 24.0;
                } else {
                    label.font_size = 16.0;
                }
                pass.request_layout();
                pass.request_render();
                pass.set_handled();
            },
        ),
        |label, pass, hovered| {
            if hovered {
                label.color = Rgba::rgb(0x73, 0x73, 0x89);
            } else {
                label.color = Rgba::rgb(0xaa, 0xaa, 0xad);
            }
            pass.request_render();
            pass.set_handled();
        },
    )
}
