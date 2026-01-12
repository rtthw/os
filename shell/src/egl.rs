//! # EGL Rendering Abstractions

use std::sync::Arc;

use anyhow::Result;



pub struct Renderer {
    pub painter: egui_glow::Painter,
    pub egui_ctx: egui::Context,
    pub gl: Arc<glow::Context>,
}

impl Renderer {
    pub fn new(display: &impl glutin::display::GlDisplay) -> Result<Self> {
        let gl = Arc::new(unsafe {
            glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s))
        });

        let painter = egui_glow::Painter::new(Arc::clone(&gl), "", None, true)?;

        let egui_ctx = egui::Context::default();
        egui_ctx.style_mut(|s| {
            s.visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);

            s.visuals.button_frame = false;
            s.visuals.collapsing_header_frame = false;

            s.visuals.panel_fill = egui::Color32::from_rgb(0x1e, 0x1e, 0x22);
            s.visuals.window_fill = egui::Color32::from_rgb(0x2b, 0x2b, 0x31);
            s.visuals.text_edit_bg_color = Some(egui::Color32::from_rgb(0x2b, 0x2b, 0x31));

            s.visuals.hyperlink_color = egui::Color32::from_rgb(0xa3, 0xa3, 0xcc);

            s.visuals.widgets.inactive.fg_stroke =
                egui::Stroke::new(1.0, egui::Color32::from_rgb(0x97, 0x97, 0xaa));
            s.visuals.widgets.hovered.fg_stroke =
                egui::Stroke::new(1.0, egui::Color32::from_rgb(0xa3, 0xa3, 0xbb));
            s.visuals.widgets.active.fg_stroke =
                egui::Stroke::new(1.0, egui::Color32::from_rgb(0xb7, 0xb7, 0xcc));
        });

        Ok(Self {
            painter,
            egui_ctx,
            gl,
        })
    }
}
