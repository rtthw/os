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

        Ok(Self {
            painter,
            egui_ctx: egui::Context::default(),
            gl,
        })
    }
}
