//! # Emulator

use {anyhow::Result, eframe::egui, std::sync::Arc};



fn main() -> Result<()> {
    eframe::run_native(
        "Emulator",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 720.0]),
            ..Default::default()
        },
        Box::new(|cc: &eframe::CreationContext| {
            let mut fonts = egui::FontDefinitions::default();

            fonts.font_data.insert(
                "icon".into(),
                Arc::new(egui_phosphor::Variant::Regular.font_data()),
            );
            fonts.families.insert(
                egui::FontFamily::Name("icon".into()),
                vec!["Ubuntu-Light".into(), "icon".into()],
            );

            fonts.font_data.insert(
                "icon-fill".into(),
                Arc::new(egui_phosphor::Variant::Fill.font_data()),
            );
            fonts.families.insert(
                egui::FontFamily::Name("icon-fill".into()),
                vec!["Ubuntu-Light".into(), "icon-fill".into()],
            );

            cc.egui_ctx.set_fonts(fonts);

            cc.egui_ctx.style_mut(|s| {
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

            Ok(Box::new(App {}))
        }),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(())
}



struct App {}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menubar")
            .show_separator_line(false)
            .default_height(30.0)
            .show(ctx, |ui| {
                let layout_ltr = egui::Layout::left_to_right(egui::Align::BOTTOM);
                let layout_rtl = egui::Layout::right_to_left(egui::Align::BOTTOM);

                ui.with_layout(layout_ltr, |ui| {
                    if ui
                        .button(icon(icons::HOUSE, IconStyle::SmallNormal))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        println!("TODO");
                    }
                    ui.with_layout(layout_rtl, |ui| {
                        if ui
                            .button(icon(icons::POWER, IconStyle::SmallNormal))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            println!("TODO");
                        }
                    });
                });
            });
        egui::SidePanel::left("sidebar")
            .show_separator_line(false)
            .default_width(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.collapsing("Drivers", |ui| {
                            if ui.button(egui::RichText::new("Terminal").weak()).clicked() {
                                println!("TODO");
                            }
                        });
                    });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::CentralPanel::default()
                .frame(egui::Frame::menu(&ctx.style()))
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.heading("Home");
                            ui.separator();

                            // self.program
                            //     .update(ui)
                            //     .expect("failed to update program");
                        });
                });
        });
    }
}



mod icons {
    pub use egui_phosphor::regular::*;
}

pub use egui_phosphor::regular::ICONS as ALL_ICONS;

pub fn icon(icon: &str, style: IconStyle) -> egui::RichText {
    egui::RichText::new(icon)
        .family(style.font_family())
        .size(style.size())
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IconStyle {
    SmallNormal,
    MediumNormal,
    LargeNormal,
    SmallFill,
    MediumFill,
    LargeFill,
}

impl IconStyle {
    pub fn font_family(&self) -> egui::FontFamily {
        egui::FontFamily::Name(
            match self {
                Self::SmallNormal | Self::MediumNormal | Self::LargeNormal => "icon",
                Self::SmallFill | Self::MediumFill | Self::LargeFill => "icon-fill",
            }
            .into(),
        )
    }

    pub const fn size(&self) -> f32 {
        match self {
            Self::SmallNormal | Self::SmallFill => 17.0,
            Self::MediumNormal | Self::MediumFill => 19.0,
            Self::LargeNormal | Self::LargeFill => 23.0,
        }
    }
}
