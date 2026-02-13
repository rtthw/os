//! # Emulator

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_error_codes;
extern crate rustc_errors;
extern crate rustc_hir as hir;
extern crate rustc_interface as interface;
extern crate rustc_session as session;
extern crate rustc_span as span;
extern crate rustc_target;

mod compiler;

use {
    abi::*,
    anyhow::Result,
    eframe::egui::{self, Rect, pos2, vec2},
    kernel::object::{Object, Ptr},
    std::{
        collections::HashMap,
        sync::{Arc, atomic::AtomicBool},
    },
};


const WORKSPACE_DIR: &str = env!("CARGO_MANIFEST_DIR");
const EXAMPLE_SRC: &str = include_str!("../../../example/src/example.rs");

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

            Ok(Box::new(App {
                program: Program::load("example", EXAMPLE_SRC.to_string(), cc.egui_ctx.clone())?,
                show_command_line: false,
                command_line_input: String::new(),
            }))
        }),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))?;

    Ok(())
}



struct App {
    program: Program,
    show_command_line: bool,
    command_line_input: String,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

                            self.program.update(ui).expect("failed to update program");
                        });
                });
        });
        if self.show_command_line {
            egui::Window::new("Command Line")
                .title_bar(false)
                .fade_in(true)
                .fade_out(true)
                .collapsible(false)
                .auto_sized()
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.command_line_input)
                            .code_editor()
                            .hint_text("Enter a command..."),
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let input = std::mem::take(&mut self.command_line_input);
                        println!("TODO: Run '{input}'");
                        self.show_command_line = false;
                    }

                    // When the command line is showing, it should always have focus.
                    response.request_focus();
                });
        } else {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Slash)) {
                self.show_command_line = true;
            }
        }
    }
}



struct Program {
    name: &'static str,
    handle: Option<ProgramHandle>,
    editing: bool,
    waiting_on_recompile: bool,
    compiling: Arc<AtomicBool>,
    latest_compile_succeeded: Arc<AtomicBool>,
    source: String,
    known_bounds: Aabb2D<f32>,
    egui_context: egui::Context,
}

impl Program {
    fn load(name: &'static str, source: String, egui_context: egui::Context) -> Result<Self> {
        let mut this = Self {
            name,
            handle: None,
            editing: false,
            waiting_on_recompile: false,
            compiling: Arc::new(AtomicBool::new(false)),
            latest_compile_succeeded: Arc::new(AtomicBool::new(true)),
            source,
            known_bounds: Aabb2D::default(),
            egui_context,
        };

        this.start_compiling();

        Ok(this)
    }

    fn start_compiling(&mut self) {
        self.waiting_on_recompile = true;
        self.compiling
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let compiling = self.compiling.clone();
        let latest_compile_succeeded = self.latest_compile_succeeded.clone();
        let content = self.source.clone();
        let input_filename = format!("{}.rs", self.name);
        let output_filename = format!("{}.so", self.name);

        std::thread::spawn(move || {
            let result = compiler::run(&content, &input_filename, &output_filename);
            if let Err(error) = &result {
                println!("ERROR: {error}");
            }
            latest_compile_succeeded.swap(result.is_ok(), std::sync::atomic::Ordering::SeqCst);
            compiling.swap(false, std::sync::atomic::Ordering::SeqCst);
        });
    }

    fn reload(&mut self) -> Result<()> {
        // We need to drop the previous shared object before reloading because `dlopen`
        // won't load the new version if there are existing references to the old one.
        drop(self.handle.take());

        let handle = unsafe {
            Object::open(format!("{WORKSPACE_DIR}/../../build/{}.so", self.name).as_str())
                .map_err(|error| anyhow::anyhow!(error.to_string_lossy().to_string()))?
        };
        let manifest = handle
            .get::<_, *const Manifest>("__MANIFEST")
            .ok_or(anyhow::anyhow!(
                "Could not find manifest for program '{}'",
                self.name,
            ))?;

        let mut view = abi::View::new(
            ((unsafe { &**manifest }).init)(),
            Box::new(FontsImpl {
                egui_context: self.egui_context.clone(),
                galley_cache: HashMap::new(),
            }),
            self.known_bounds.size(),
        );

        let mut render = Render::default();
        view.render(&mut render);

        self.handle = Some(ProgramHandle {
            view,
            render,
            _manifest: manifest,
            _handle: handle,
        });

        Ok(())
    }

    fn update(&mut self, ui: &mut egui::Ui) -> Result<()> {
        if self.compiling.load(std::sync::atomic::Ordering::Relaxed) {
            ui.centered_and_justified(|ui| {
                ui.spinner();
            });
            return Ok(());
        }

        let compile_success = self
            .latest_compile_succeeded
            .load(std::sync::atomic::Ordering::Relaxed);

        if self.waiting_on_recompile && compile_success {
            self.waiting_on_recompile = false;
            self.reload()?;
        }

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(ui.available_height());

            if self.editing {
                ui.allocate_ui_with_layout(
                    vec2(
                        ui.available_size_before_wrap().x,
                        ui.spacing().interact_size.y,
                    ),
                    egui::Layout::right_to_left(egui::Align::Center).with_main_wrap(true),
                    |ui| {
                        ui.add_space(IconStyle::SmallNormal.size());
                        if ui.button("Confirm").clicked() {
                            self.editing = false;
                            self.start_compiling();
                        }
                        if ui.button("Cancel").clicked() {
                            self.editing = false;
                        }
                    },
                );
                ui.separator();
                ui.add(
                    egui::TextEdit::multiline(&mut self.source)
                        .code_editor()
                        .font(egui::FontId::monospace(20.0))
                        .desired_width(ui.available_width()),
                );
                return;
            }
            ui.allocate_ui_with_layout(
                vec2(
                    ui.available_size_before_wrap().x,
                    ui.spacing().interact_size.y,
                ),
                egui::Layout::right_to_left(egui::Align::Center).with_main_wrap(true),
                |ui| {
                    ui.add_space(IconStyle::SmallNormal.size());
                    if ui
                        .button(icon(icons::PENCIL, IconStyle::SmallNormal).weak())
                        .clicked()
                    {
                        self.editing = true;
                    }
                },
            );
            ui.separator();
            if !compile_success {
                ui.centered_and_justified(|ui| {
                    ui.heading("Compilation failed, see logs");
                });
                return;
            }

            let handle = self.handle.as_mut().unwrap();
            let view = &mut handle.view;
            let render = &mut handle.render;

            let mut rendered = false;
            for event in ui.input(|i| {
                i.filtered_events(&egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                })
            }) {
                match event {
                    egui::Event::Key {
                        key,
                        pressed,
                        modifiers,
                        ..
                    } => {
                        let Some(key) = egui_key_to_key(key, modifiers) else {
                            continue;
                        };
                        view.handle_keyboard_event(if pressed {
                            KeyboardEvent::Down { key }
                        } else {
                            KeyboardEvent::Up { key }
                        });
                        view.render(render);
                        rendered = true;
                    }
                    egui::Event::PointerMoved(pos) => {
                        let pos = Xy::new(pos.x, pos.y);
                        view.handle_pointer_event(PointerEvent::Move {
                            position: pos - self.known_bounds.position(),
                        });
                        view.render(render);
                        rendered = true;
                    }
                    egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        ..
                    } => {
                        let position = Xy::new(pos.x, pos.y) - self.known_bounds.position();
                        let button = match button {
                            egui::PointerButton::Primary => PointerButton::Primary,
                            egui::PointerButton::Secondary => PointerButton::Secondary,
                            egui::PointerButton::Middle => PointerButton::Auxiliary,
                            egui::PointerButton::Extra1 => PointerButton::Back,
                            egui::PointerButton::Extra2 => PointerButton::Forward,
                        };
                        view.handle_pointer_event(if pressed {
                            PointerEvent::Down { button, position }
                        } else {
                            PointerEvent::Up { button }
                        });
                        view.render(render);
                        rendered = true;
                    }
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta,
                        ..
                    } => {
                        view.handle_pointer_event(PointerEvent::Scroll {
                            delta: ScrollDelta::Lines(Xy::new(delta.x, delta.y)),
                        });
                        view.render(render);
                        rendered = true;
                    }
                    _ => {}
                }
            }

            let window_bounds = rect_to_aabb2d(ui.available_rect_before_wrap());
            if self.known_bounds != window_bounds {
                self.known_bounds = window_bounds;
                view.resize_window(window_bounds.size());
                view.render(render);
                rendered = true;
            }

            if view.animating() {
                if !rendered {
                    view.render(render);
                }
                ui.ctx().request_repaint();
            }

            if ui.ui_contains_pointer() {
                ui.ctx()
                    .set_cursor_icon(abi_to_egui_cursor_icon(view.cursor_icon()));
            }

            {
                let mut bounds = self.known_bounds;
                let mut text = String::new();
                let mut font_size = 16.0;
                let mut foreground_color = Rgba::WHITE;
                let mut background_color = Rgba::BLACK;
                let mut border_color = Rgba::NONE;
                let mut border_width = 0.0;

                let painter = ui
                    .painter()
                    .with_clip_rect(aabb2d_to_rect(self.known_bounds));
                for command in render.commands.iter() {
                    if !matches!(command, RenderCommand::DrawChar(_)) && !text.is_empty() {
                        let pos = bounds.position() + self.known_bounds.position();
                        painter
                            .with_clip_rect(aabb2d_to_rect(
                                bounds.translate(self.known_bounds.position()),
                            ))
                            .text(
                                pos2(pos.x, pos.y),
                                egui::Align2::LEFT_TOP,
                                std::mem::take(&mut text),
                                egui::FontId {
                                    size: font_size,
                                    family: egui::FontFamily::Proportional,
                                },
                                rgba_to_color32(foreground_color),
                            );
                    }

                    match command {
                        RenderCommand::DrawChar(ch) => text.push(*ch),
                        RenderCommand::DrawQuad => {
                            painter.rect(
                                aabb2d_to_rect(bounds.translate(self.known_bounds.position())),
                                3,
                                rgba_to_color32(background_color),
                                egui::Stroke::new(border_width, rgba_to_color32(border_color)),
                                egui::StrokeKind::Inside,
                            );
                        }
                        RenderCommand::SetBounds(aabb2d) => bounds = *aabb2d,
                        RenderCommand::SetForegroundColor(rgba) => foreground_color = *rgba,
                        RenderCommand::SetBackgroundColor(rgba) => background_color = *rgba,
                        RenderCommand::SetBorderColor(rgba) => border_color = *rgba,
                        RenderCommand::SetBorderWidth(width) => border_width = *width,
                        RenderCommand::SetFontSize(size) => font_size = *size,
                    }
                }

                // We need to manually check the text length because the render commands could
                // end with a `DrawChar`, which wouldn't be checked in the loop above.
                if !text.is_empty() {
                    let pos = bounds.position() + self.known_bounds.position();
                    painter
                        .with_clip_rect(aabb2d_to_rect(
                            bounds.translate(self.known_bounds.position()),
                        ))
                        .text(
                            pos2(pos.x, pos.y),
                            egui::Align2::LEFT_TOP,
                            std::mem::take(&mut text),
                            egui::FontId {
                                size: font_size,
                                family: egui::FontFamily::Proportional,
                            },
                            rgba_to_color32(foreground_color),
                        );
                }
            }
        });

        Ok(())
    }
}

struct ProgramHandle {
    view: View,
    render: Render,
    _manifest: Ptr<*const Manifest>,
    _handle: Object,
}



struct FontsImpl {
    egui_context: egui::Context,
    galley_cache: HashMap<String, Arc<egui::text::Galley>>,
}

impl Fonts for FontsImpl {
    fn measure_text(
        &mut self,
        _id: u64,
        text: &str,
        max_advance: Option<f32>,
        font_size: f32,
        _line_height: LineHeight,
        _font_style: FontStyle,
        alignment: TextAlignment,
        wrap_mode: TextWrapMode,
    ) -> Xy<f32> {
        let run_layout = || {
            self.egui_context.fonts_mut(|fonts| {
                fonts.layout_job(egui::text::LayoutJob {
                    text: text.to_string(),
                    sections: vec![egui::text::LayoutSection {
                        leading_space: 0.0,
                        byte_range: 0..text.len(),
                        format: egui::TextFormat::simple(
                            egui::FontId {
                                size: font_size,
                                family: egui::FontFamily::Proportional,
                            },
                            egui::Color32::WHITE,
                        ),
                    }],
                    wrap: egui::text::TextWrapping {
                        max_width: max_advance.unwrap_or(f32::INFINITY),
                        max_rows: if wrap_mode == TextWrapMode::Wrap {
                            usize::MAX
                        } else {
                            1
                        },
                        break_anywhere: false,
                        overflow_character: Default::default(),
                    },
                    first_row_min_height: 0.0,
                    break_on_newline: true,
                    halign: match alignment {
                        TextAlignment::Start => egui::Align::Min,
                        TextAlignment::End => egui::Align::Max,
                        TextAlignment::Left => egui::Align::LEFT,
                        TextAlignment::Center => egui::Align::Center,
                        TextAlignment::Right => egui::Align::RIGHT,
                        TextAlignment::Justify => egui::Align::Min,
                    },
                    justify: alignment == TextAlignment::Justify,
                    round_output_to_gui: true,
                })
            })
        };

        let galley = self
            .galley_cache
            .entry(text.to_string())
            .or_insert_with(|| run_layout());

        if galley.text() != text
            || galley.job.sections.first().unwrap().format.font_id.size != font_size
        {
            *galley = run_layout();
            // println!("{text} @ {font_size} = {:?}", galley.rect.size());
        }

        let rect = galley.rect;

        Xy::new(rect.width(), rect.height())
    }
}

fn rgba_to_color32(color: abi::Rgba<u8>) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(color.r, color.g, color.b, color.a)
}

fn rect_to_aabb2d(bounds: Rect) -> abi::Aabb2D<f32> {
    abi::Aabb2D {
        min: Xy::new(bounds.min.x, bounds.min.y),
        max: Xy::new(bounds.max.x, bounds.max.y),
    }
}

fn aabb2d_to_rect(bounds: abi::Aabb2D<f32>) -> Rect {
    Rect::from_min_max(
        pos2(bounds.min.x, bounds.min.y),
        pos2(bounds.max.x, bounds.max.y),
    )
}

fn abi_to_egui_cursor_icon(value: CursorIcon) -> egui::CursorIcon {
    match value {
        CursorIcon::AllScroll => egui::CursorIcon::AllScroll,
        CursorIcon::Grab => egui::CursorIcon::Grab,
        CursorIcon::Grabbing => egui::CursorIcon::Grabbing,
        CursorIcon::Help => egui::CursorIcon::Help,
        CursorIcon::NoDrop => egui::CursorIcon::NoDrop,
        CursorIcon::PointingHand => egui::CursorIcon::PointingHand,
        CursorIcon::SplitH => egui::CursorIcon::ResizeHorizontal,
        CursorIcon::SplitV => egui::CursorIcon::ResizeVertical,
        CursorIcon::IBeam => egui::CursorIcon::Text,
        CursorIcon::ZoomIn => egui::CursorIcon::ZoomIn,
        CursorIcon::ZoomOut => egui::CursorIcon::ZoomOut,
        _ => egui::CursorIcon::Default,
    }
}

fn egui_key_to_key(key: egui::Key, mods: egui::Modifiers) -> Option<Key> {
    Some(match key {
        egui::Key::Space => Key::Space,
        egui::Key::Tab => Key::Tab,
        egui::Key::Enter => Key::Enter,
        egui::Key::Backspace => Key::Backspace,
        egui::Key::Delete => Key::Delete,

        egui::Key::ArrowUp => Key::ArrowUp,
        egui::Key::ArrowDown => Key::ArrowDown,
        egui::Key::ArrowLeft => Key::ArrowLeft,
        egui::Key::ArrowRight => Key::ArrowRight,

        egui::Key::PageUp => Key::PageUp,
        egui::Key::PageDown => Key::PageDown,

        other => Key::Char(if mods.shift {
            match other {
                egui::Key::Num0 => ')',
                egui::Key::Num1 => '!',
                egui::Key::Num2 => '@',
                egui::Key::Num3 => '#',
                egui::Key::Num4 => '$',
                egui::Key::Num5 => '%',
                egui::Key::Num6 => '^',
                egui::Key::Num7 => '&',
                egui::Key::Num8 => '*',
                egui::Key::Num9 => '(',
                egui::Key::Minus => '_',
                egui::Key::Equals => '+',
                egui::Key::A => 'A',
                egui::Key::B => 'B',
                egui::Key::C => 'C',
                egui::Key::D => 'D',
                egui::Key::E => 'E',
                egui::Key::F => 'F',
                egui::Key::G => 'G',
                egui::Key::H => 'H',
                egui::Key::I => 'I',
                egui::Key::J => 'J',
                egui::Key::K => 'K',
                egui::Key::L => 'L',
                egui::Key::M => 'M',
                egui::Key::N => 'N',
                egui::Key::O => 'O',
                egui::Key::P => 'P',
                egui::Key::Q => 'Q',
                egui::Key::R => 'R',
                egui::Key::S => 'S',
                egui::Key::T => 'T',
                egui::Key::U => 'U',
                egui::Key::V => 'V',
                egui::Key::W => 'W',
                egui::Key::X => 'X',
                egui::Key::Y => 'Y',
                egui::Key::Z => 'Z',
                _ => None?,
            }
        } else {
            match other {
                egui::Key::Num0 => '0',
                egui::Key::Num1 => '1',
                egui::Key::Num2 => '2',
                egui::Key::Num3 => '3',
                egui::Key::Num4 => '4',
                egui::Key::Num5 => '5',
                egui::Key::Num6 => '6',
                egui::Key::Num7 => '7',
                egui::Key::Num8 => '8',
                egui::Key::Num9 => '9',
                egui::Key::Minus => '-',
                egui::Key::Equals => '=',
                egui::Key::A => 'a',
                egui::Key::B => 'b',
                egui::Key::C => 'c',
                egui::Key::D => 'd',
                egui::Key::E => 'e',
                egui::Key::F => 'f',
                egui::Key::G => 'g',
                egui::Key::H => 'h',
                egui::Key::I => 'i',
                egui::Key::J => 'j',
                egui::Key::K => 'k',
                egui::Key::L => 'l',
                egui::Key::M => 'm',
                egui::Key::N => 'n',
                egui::Key::O => 'o',
                egui::Key::P => 'p',
                egui::Key::Q => 'q',
                egui::Key::R => 'r',
                egui::Key::S => 's',
                egui::Key::T => 't',
                egui::Key::U => 'u',
                egui::Key::V => 'v',
                egui::Key::W => 'w',
                egui::Key::X => 'x',
                egui::Key::Y => 'y',
                egui::Key::Z => 'z',
                _ => None?,
            }
        }),
    })
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



#[allow(unused)]
#[unsafe(export_name = "__ui_Label__children_ids")]
pub extern "Rust" fn __label_children_ids(_label: &Label) -> Vec<u64> {
    Vec::new()
}

#[allow(unused)]
#[unsafe(export_name = "__ui_Label__render")]
pub extern "Rust" fn __label_render(label: &mut Label, pass: &mut RenderPass<'_>) {
    pass.fill_text(
        label.text.clone(),
        pass.bounds(),
        label.color,
        label.font_size,
    );
}

#[allow(unused)]
#[unsafe(export_name = "__ui_Label__layout")]
pub extern "Rust" fn __label_layout(_label: &mut Label, _pass: &mut LayoutPass<'_>) {}

#[allow(unused)]
#[unsafe(export_name = "__ui_Label__measure")]
pub extern "Rust" fn __label_measure(
    label: &mut Label,
    context: &mut MeasureContext<'_>,
    axis: Axis,
    length_request: LengthRequest,
    cross_length: Option<f32>,
) -> f32 {
    let id = context.id();
    let fonts = context.fonts_mut();
    // For exact measurements, we round up so the `FontsImpl` doesn't wrap
    // unnecessarily.
    let max_advance = match axis {
        Axis::Horizontal => match length_request {
            LengthRequest::MinContent | LengthRequest::MaxContent => None,
            LengthRequest::FitContent(space) => Some(space),
        },
        Axis::Vertical => None,
    };
    let used_size = fonts.measure_text(
        id,
        &label.text,
        max_advance,
        label.font_size,
        label.line_height,
        label.font_style,
        label.alignment,
        label.wrap_mode,
    );

    used_size.value_for_axis(axis)
}
