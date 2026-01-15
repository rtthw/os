pub mod cursor;
pub mod egl;
pub mod input;
pub mod log;
pub mod object;

use std::{
    collections::HashMap,
    ffi::OsString,
    io::{BufRead as _, Read as _, Write as _},
    num::NonZeroU32,
    os::fd::AsRawFd as _,
    ptr::NonNull,
    str::FromStr as _,
    sync::Arc,
    time::Instant,
};

use {
    ::log::{debug, error, info, trace, warn},
    anyhow::{Result, bail},
    drm::{Device, control::Device as ControlDevice},
    egui::{Pos2, Rect, pos2, vec2},
    gbm::AsRaw as _,
    glow::HasContext as _,
    glutin::{
        config::GlConfig as _,
        display::{GetGlDisplay as _, GlDisplay as _},
        prelude::{NotCurrentGlContext as _, PossiblyCurrentGlContext as _},
        surface::GlSurface as _,
    },
    kernel::{
        epoll::{Event, EventPoll},
        file::File,
    },
};

use crate::{
    cursor::{CursorData, CursorIcon},
    object::Object,
};



fn main() -> Result<()> {
    let startup_time = Instant::now();

    unsafe {
        std::env::set_var("HOME", "/home");
        std::env::set_var("PATH", "/home/.cargo/bin:/usr/bin");
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    log::Logger::default().init()?;

    info!("Running test program...");

    let testing_obj = unsafe { Object::open("/usr/bin/testing")? };
    let manifest = testing_obj
        .get::<_, *mut abi::Manifest>("__MANIFEST")
        .ok_or(anyhow::anyhow!("Could not find manifest for test program"))?;
    'run_test_program: {
        let man = unsafe { &**manifest };
        if man.abi_version != abi::VERSION {
            warn!(
                "Test program was built for a different ABI than the one currently running: \
                expected {}, got {}",
                abi::VERSION,
                man.abi_version,
            );
            break 'run_test_program;
        }

        (man.entry_point)();
    }

    info!("Starting shell...");

    std::thread::sleep(std::time::Duration::from_secs(1));

    let gpu = GraphicsCard::open("/dev/dri/card0")?;

    let display = unsafe {
        glutin::api::egl::display::Display::new(raw_window_handle::RawDisplayHandle::Gbm(
            raw_window_handle::GbmDisplayHandle::new(NonNull::new(gpu.as_raw() as *mut _).unwrap()),
        ))
    }
    .expect("Failed to create display");

    let config = unsafe {
        display.find_configs(
            glutin::config::ConfigTemplateBuilder::default()
                .with_surface_type(glutin::config::ConfigSurfaceTypes::WINDOW)
                .build(),
        )
    }
    .unwrap()
    .reduce(|config, acc| {
        debug!(
            "{:?}, {:?}, {:?}, SRGB={}, HWACC={}",
            config.api(),
            config.config_surface_types(),
            config.color_buffer_type(),
            config.srgb_capable(),
            config.hardware_accelerated()
        );

        if config.num_samples() > acc.num_samples() {
            config
        } else {
            acc
        }
    })
    .expect("no available GL configs");

    let context_attributes = glutin::context::ContextAttributesBuilder::new().build(None);
    let fallback_context_attributes = glutin::context::ContextAttributesBuilder::new()
        .with_context_api(glutin::context::ContextApi::Gles(None))
        .build(None);
    let context = unsafe {
        display
            .create_context(&config, &context_attributes)
            .unwrap_or_else(|_| {
                display
                    .create_context(&config, &fallback_context_attributes)
                    .expect("failed to create context")
            })
    };

    trace!(target: "gpu", "Setting DRM client capabilities...");

    gpu.set_client_capability(drm::ClientCapability::UniversalPlanes, true)
        .expect("unable to request gpu.UniversalPlanes capability");
    gpu.set_client_capability(drm::ClientCapability::Atomic, true)
        .expect("unable to request gpu.Atomic capability");
    gpu.set_client_capability(drm::ClientCapability::CursorPlaneHotspot, true)
        .expect("unable to request gpu.Atomic capability");

    trace!(target: "gpu", "Preparing outputs...");

    let output = match gpu.prepare_output(&config, context) {
        Ok(output) => output,
        Err(error) => {
            bail!(
                "\x1b[31mERROR\x1b[0m \x1b[2m(shell)\x1b[0m: \
                Failed to prepare outputs: {error}",
            );
        }
    };

    let cursor_width = gpu
        .get_driver_capability(drm::DriverCapability::CursorWidth)
        .unwrap_or(64);
    let cursor_height = gpu
        .get_driver_capability(drm::DriverCapability::CursorHeight)
        .unwrap_or(64);
    let cursor_hotspot;
    let mut cursor_data = HashMap::new();
    #[allow(deprecated)]
    let cursor_buffer = {
        let data = cursor_data
            .entry(CursorIcon::Default)
            .or_insert_with(|| CursorData::load_or_fallback("/usr/share/cursors/default/default"))
            .get_image(1, startup_time.elapsed().as_millis() as _);

        let mut buffer: gbm::BufferObject<()> = gpu.create_buffer_object(
            cursor_width as _,
            cursor_height as _,
            gbm::Format::Argb8888,
            gbm::BufferObjectFlags::CURSOR | gbm::BufferObjectFlags::WRITE,
        )?;

        println!(
            "IMAGE: {}x{}, {}",
            data.width,
            data.height,
            data.pixels_rgba.len()
        );

        buffer.map_mut(0, 0, data.width, data.height, |map| {
            map.buffer_mut()
                .chunks_exact_mut(cursor_width as usize * 4)
                .zip(data.pixels_rgba.chunks_exact(data.width as usize * 4))
                .for_each(|(dst, src)| dst[..src.len()].copy_from_slice(src));
        })?;

        cursor_hotspot = (data.xhot as _, data.yhot as _);

        if gpu
            .set_cursor2(output.crtc, Some(&buffer), cursor_hotspot)
            .is_err()
        {
            gpu.set_cursor(output.crtc, Some(&buffer))?;
        }

        buffer
    };

    // let cursor_plane = gpu.plane_handles()?.iter()
    //     .find_map(|plane| {
    //         let info = gpu.get_plane(*plane).ok()?;
    //         let prop = gpu.get_properties(*plane).ok()?.iter().find_map(|prop| {
    //             let info = gpu.get_property(*prop.0).ok()?;
    //             (info.name() == c"type").then_some({
    //                 let value_type = info.value_type();
    //                 let drm::control::property::Value::Enum(value)
    //                     = value_type.convert_value(*prop.1)
    //                 else {
    //                     return None;
    //                 };
    //                 value?.value()
    //             })
    //         })?;

    //         (prop == drm::control::PlaneType::Cursor as u64&& info.crtc() ==
    // Some(output.crtc))             .then_some(info)
    //     })
    //     .expect("failed to find cursor plane")
    //     .handle();

    let this_obj = unsafe { Object::open_this().expect("should be able to open shell binary") };

    let stdin = std::io::stdin();
    unsafe {
        assert_ne!(
            libc::fcntl(stdin.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK),
            -1
        );
    }

    let mut event_loop = EventLoop::new()?;

    event_loop.add_source(gpu.clone(), |shell, drm_event| {
        if let drm::control::Event::PageFlip(_event) = drm_event {
            shell.render()?;
        } else {
            trace!("Unknown DRM event occurred");
        }

        Ok(())
    })?;

    for (path, device) in evdev::enumerate() {
        let name = device.name().unwrap_or("Unnamed Device").to_string();

        let abs_info = device.get_absinfo().map(|info| info.collect::<Vec<_>>());

        debug!(
            target: "dev",
            "{}\n\
            \t.name: {}\n\
            \t.physical_path: {}\n\
            \t.properties: {:?}\n\
            \t.misc_properties: {:?}\n\
            \t.supported_events: {:?}\n\
            \t.supported_keys: {:?}\n\
            \t.supported_absolute_axes: {:?}\n\
            \t.supported_relative_axes: {:?}\n\
            \t.abs_info: {:?}",
            path.display(),
            &name,
            device.physical_path().unwrap_or("NONE"),
            device.properties(),
            device.misc_properties(),
            device.supported_events(),
            device.supported_keys(),
            device.supported_absolute_axes(),
            device.supported_relative_axes(),
            &abs_info,
        );

        let max_abs_x = abs_info
            .as_ref()
            .map(|vals| {
                vals.iter()
                    .find(|val| val.0 == evdev::AbsoluteAxisCode::ABS_X)
                    .map(|val| val.1.maximum())
                    .unwrap_or(0)
            })
            .unwrap_or(0) as f32;
        let max_abs_y = abs_info
            .as_ref()
            .map(|vals| {
                vals.iter()
                    .find(|val| val.0 == evdev::AbsoluteAxisCode::ABS_Y)
                    .map(|val| val.1.maximum())
                    .unwrap_or(0)
            })
            .unwrap_or(0) as f32;

        event_loop.add_source(
            input::InputSource::new(device)?,
            move |shell, input_event| {
                match input_event.event_type() {
                    evdev::EventType::ABSOLUTE => {
                        match evdev::AbsoluteAxisCode(input_event.code()) {
                            evdev::AbsoluteAxisCode::ABS_X => {
                                let abs_x = input_event.value() as f32;
                                if abs_x == 0.0 {
                                    shell.input_state.mouse_pos.x = 0.0;
                                } else {
                                    shell.input_state.mouse_pos.x =
                                        shell.output.width() as f32 / (max_abs_x / abs_x);
                                }
                                shell
                                    .input_state
                                    .events
                                    .push(egui::Event::PointerMoved(shell.input_state.mouse_pos));
                            }
                            evdev::AbsoluteAxisCode::ABS_Y => {
                                let abs_y = input_event.value() as f32;
                                if abs_y == 0.0 {
                                    shell.input_state.mouse_pos.y = 0.0;
                                } else {
                                    shell.input_state.mouse_pos.y =
                                        shell.output.height() as f32 / (max_abs_y / abs_y);
                                }
                                shell
                                    .input_state
                                    .events
                                    .push(egui::Event::PointerMoved(shell.input_state.mouse_pos));
                            }
                            _ => {}
                        }
                    }
                    evdev::EventType::RELATIVE => {
                        match evdev::RelativeAxisCode(input_event.code()) {
                            evdev::RelativeAxisCode::REL_X => {
                                let movement = input_event.value() as f32;
                                shell.input_state.mouse_pos.x += movement;
                                shell
                                    .input_state
                                    .events
                                    .push(egui::Event::PointerMoved(shell.input_state.mouse_pos));
                                shell
                                    .input_state
                                    .events
                                    .push(egui::Event::MouseMoved(vec2(movement, 0.0)));
                            }
                            evdev::RelativeAxisCode::REL_Y => {
                                let movement = input_event.value() as f32;
                                shell.input_state.mouse_pos.y += movement;
                                shell
                                    .input_state
                                    .events
                                    .push(egui::Event::PointerMoved(shell.input_state.mouse_pos));
                                shell
                                    .input_state
                                    .events
                                    .push(egui::Event::MouseMoved(vec2(0.0, movement)));
                            }
                            evdev::RelativeAxisCode::REL_WHEEL => {
                                shell.input_state.events.push(egui::Event::MouseWheel {
                                    unit: egui::MouseWheelUnit::Line,
                                    delta: vec2(0.0, input_event.value() as f32),
                                    modifiers: shell.input_state.key_modifiers,
                                });
                            }
                            _ => {}
                        }
                    }
                    evdev::EventType::KEY => match evdev::KeyCode(input_event.code()) {
                        evdev::KeyCode::BTN_LEFT => {
                            shell.input_state.events.push(egui::Event::PointerButton {
                                pos: shell.input_state.mouse_pos,
                                button: egui::PointerButton::Primary,
                                pressed: input_event.value() == 1,
                                modifiers: shell.input_state.key_modifiers,
                            });
                        }
                        evdev::KeyCode::BTN_RIGHT => {
                            shell.input_state.events.push(egui::Event::PointerButton {
                                pos: shell.input_state.mouse_pos,
                                button: egui::PointerButton::Secondary,
                                pressed: input_event.value() == 1,
                                modifiers: shell.input_state.key_modifiers,
                            });
                        }

                        evdev::KeyCode::KEY_LEFTCTRL | evdev::KeyCode::KEY_RIGHTCTRL => {
                            shell.input_state.key_modifiers.ctrl = input_event.value() == 1;
                            shell.input_state.key_modifiers.command = input_event.value() == 1;
                        }
                        evdev::KeyCode::KEY_LEFTSHIFT | evdev::KeyCode::KEY_RIGHTSHIFT => {
                            shell.input_state.key_modifiers.shift = input_event.value() == 1;
                        }
                        evdev::KeyCode::KEY_LEFTALT | evdev::KeyCode::KEY_RIGHTALT => {
                            shell.input_state.key_modifiers.alt = input_event.value() == 1;
                        }

                        other => {
                            let pressed = input_event.value() == 1;
                            if pressed {
                                let shift = shell.input_state.key_modifiers.shift;
                                if let Some(ch) = evdev_keycode_to_char(other, shift) {
                                    shell
                                        .input_state
                                        .events
                                        .push(egui::Event::Text(ch.to_string()));
                                }
                            }
                            if let Some(key) = evdev_keycode_to_egui_key(other) {
                                shell.input_state.events.push(egui::Event::Key {
                                    key,
                                    physical_key: Some(key),
                                    pressed,
                                    repeat: false,
                                    modifiers: shell.input_state.key_modifiers,
                                });
                            }
                        }
                    },
                    _ => {}
                }

                Ok(())
            },
        )?;
    }

    gpu.debug_info("/dev/dri/card0");

    let mut shell = Shell {
        startup_time,
        gpu: gpu.clone(),
        output,
        current_dir: std::env::current_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
        input_state: InputState {
            mouse_pos: pos2(0.0, 0.0),
            events: Vec::with_capacity(2),
            key_modifiers: egui::Modifiers::NONE,
        },
        input_buffer: String::new(),
        cursor_width,
        cursor_hotspot,
        cursor_icon: CursorIcon::Default,
        cursor_data,
        cursor_buffer,
    };

    shell.render()?;

    print!("\x1b[2m {} }} \x1b[0m", &shell.current_dir);

    std::io::stdout().flush().unwrap();

    event_loop.run(&mut shell, 16, |shell| {
        shell.render().unwrap();
        if stdin.lock().read(&mut []).is_err() {
            return;
        }

        let mut line = String::new();

        let Ok(_) = stdin.lock().take(256).read_line(&mut line) else {
            return;
        };

        let line = line.trim().to_string();

        if line.is_empty() {
            return;
        }

        let args = line.split(' ').collect::<Vec<_>>();
        let args_os: Vec<OsString> = args
            .iter()
            .map(|item| OsString::from_str(item).unwrap())
            .collect();

        match args[0] {
            "cd" => {
                if let Err(error) = std::env::set_current_dir(args[1]) {
                    println!("{error}");
                } else {
                    shell.current_dir = std::env::current_dir()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string();
                }
            }
            "env" => {
                if args.len() == 1 {
                    for (name, value) in std::env::vars() {
                        println!("{name} = {value}");
                    }
                } else {
                    match std::env::var(args[1]) {
                        Ok(value) => {
                            println!("{value}")
                        }
                        Err(error) => {
                            println!("{error}");
                        }
                    }
                }
            }
            "exit" => {
                std::process::exit(0);
            }
            "sym" => {
                // The type doesn't matter in this case (we're just printing debug info).
                match this_obj.get_untyped(args[1]) {
                    Some(ptr) => {
                        println!("{ptr:?}")
                    }
                    None => {
                        println!("Symbol '{}' not found", args[1]);
                    }
                }
            }
            // "clear" => 'handle_clear: {
            //     if args.len() >= 4 {
            //         let Ok(r) = u8::from_str_radix(args[1], 10) else {
            //             println!("Invalid red channel: {}", args[1]);
            //             break 'handle_clear;
            //         };
            //         let Ok(g) = u8::from_str_radix(args[2], 10) else {
            //             println!("Invalid green channel: {}", args[2]);
            //             break 'handle_clear;
            //         };
            //         let Ok(b) = u8::from_str_radix(args[3], 10) else {
            //             println!("Invalid blue channel: {}", args[3]);
            //             break 'handle_clear;
            //         };
            //         let a = {
            //             if args.len() == 5 {
            //                 let Ok(a) = u8::from_str_radix(args[4], 10) else {
            //                     println!("Invalid alpha channel: {}", args[4]);
            //                     break 'handle_clear;
            //                 };
            //                 a
            //             } else {
            //                 255
            //             }
            //         };

            //         clear_color = [b, g, r, a];

            //         'map_outputs: for output in &mut outputs {
            //             let Ok(mut map) = gpu.map_dumb_buffer(&mut output.db) else {
            //                 println!(
            //                     "\x1b[31mERROR\x1b[0m \x1b[2m(shell.clear)\x1b[0m: \
            //                     Failed to map output buffer",
            //                 );
            //                 continue 'map_outputs;
            //             };
            //             for pixel in map.chunks_exact_mut(4) {
            //                 pixel[0] = clear_color[0];
            //                 pixel[1] = clear_color[1];
            //                 pixel[2] = clear_color[2];
            //                 pixel[3] = clear_color[3];
            //             }
            //             if let Err(error) = gpu.set_crtc(
            //                 output.crtc,
            //                 Some(output.fb),
            //                 (0, 0),
            //                 &[output.conn],
            //                 Some(output.mode),
            //             ) {
            //                 println!(
            //                     "\x1b[31mERROR\x1b[0m \x1b[2m(shell.clear)\x1b[0m: \
            //                     Failed to set CRTC: {error}",
            //                 );
            //             }
            //         }
            //     } else {
            //         println!("Invalid arguments");
            //     }
            // }
            _ => {
                match std::process::Command::new(args[0])
                    .args(&args_os[1..])
                    .output()
                {
                    Ok(output) => {
                        println!("{}", String::from_utf8(output.stdout).unwrap());
                        println!("{}", String::from_utf8(output.stderr).unwrap());
                    }
                    Err(error) => {
                        println!("{error}");
                    }
                }
            }
        }

        print!("\x1b[2m {} }} \x1b[0m", &shell.current_dir);

        std::io::stdout().flush().unwrap();
    })
}



pub struct Shell {
    startup_time: Instant,
    gpu: GraphicsCard,
    current_dir: String,
    output: Output,
    input_state: InputState,
    input_buffer: String,
    cursor_width: u64,
    cursor_hotspot: (i32, i32),
    cursor_icon: CursorIcon,
    cursor_data: HashMap<CursorIcon, CursorData>,
    cursor_buffer: gbm::BufferObject<()>,
}

impl Shell {
    fn render(&mut self) -> Result<()> {
        self.output
            .context
            .make_current(&self.output.surface)
            .unwrap();

        #[allow(deprecated)]
        self.gpu.move_cursor(
            self.output.crtc,
            (
                self.input_state.mouse_pos.x as _,
                self.input_state.mouse_pos.y as _,
            ),
        )?;

        let (width, height) = self.output.mode.size();
        let size = vec2(width as _, height as _);
        let rect = Rect::from_min_size(Pos2::ZERO, size);

        let raw_input = egui::RawInput {
            viewport_id: egui::ViewportId::ROOT,
            viewports: std::iter::once((
                egui::ViewportId::ROOT,
                egui::ViewportInfo {
                    parent: None,
                    title: None,
                    events: Vec::new(),
                    native_pixels_per_point: Some(1.0),
                    monitor_size: Some(size),
                    inner_rect: Some(rect),
                    outer_rect: Some(rect),
                    minimized: Some(false),
                    maximized: Some(true),
                    fullscreen: Some(true),
                    focused: Some(true),
                },
            ))
            .collect(),
            screen_rect: Some(rect),
            max_texture_side: None,
            time: Some(self.startup_time.elapsed().as_secs_f64()),
            predicted_dt: 1.0 / 60.0,
            modifiers: self.input_state.key_modifiers,
            events: self.input_state.events.drain(..).collect(),
            hovered_files: Vec::new(),
            dropped_files: Vec::new(),
            focused: true,
            system_theme: Some(egui::Theme::Dark),
            safe_area_insets: None,
        };

        let full_output = self.output.renderer.egui_ctx.run(raw_input, |ctx| {
            egui::TopBottomPanel::top("menubar")
                .show_separator_line(false)
                .default_height(30.0)
                .show(ctx, |ui| {
                    let layout_ltr = egui::Layout::left_to_right(egui::Align::BOTTOM);
                    let layout_rtl = egui::Layout::right_to_left(egui::Align::BOTTOM);

                    ui.with_layout(layout_ltr, |ui| {
                        if ui
                            .button(egl::icon(egl::icons::HOUSE, egl::IconStyle::SmallNormal))
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            println!("TODO");
                        }
                        ui.with_layout(layout_rtl, |ui| {
                            if ui
                                .button(egl::icon(egl::icons::POWER, egl::IconStyle::SmallNormal))
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

                                let resp = ui.add(
                                    egui::TextEdit::singleline(&mut self.input_buffer)
                                        .hint_text("Enter a command..."),
                                );
                                if resp.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    let line: String = self.input_buffer.drain(..).collect();
                                    println!("{}", line);
                                }

                                ui.horizontal_wrapped(|ui| {
                                    for (name, icon) in egl::ALL_ICONS {
                                        ui.label(egl::icon(*icon, egl::IconStyle::LargeNormal))
                                            .on_hover_text(*name);
                                    }
                                });
                            });
                    });
            });
        });
        let clipped_primitives = self
            .output
            .renderer
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        unsafe {
            self.output.renderer.gl.clear_color(0.1, 0.1, 0.1, 1.0);
        }

        self.output.renderer.painter.paint_and_update_textures(
            [width as _, height as _],
            full_output.pixels_per_point,
            &clipped_primitives,
            &full_output.textures_delta,
        );

        unsafe {
            self.output.renderer.gl.finish();
        }

        let next_icon = CursorIcon::from(full_output.platform_output.cursor_icon);

        if self.cursor_icon != next_icon {
            self.cursor_icon = next_icon;

            let data = self
                .cursor_data
                .entry(self.cursor_icon)
                .or_insert_with(|| {
                    CursorData::load_or_fallback(&format!(
                        "/usr/share/cursors/default/{}",
                        self.cursor_icon.name(),
                    ))
                })
                .get_image(1, self.startup_time.elapsed().as_millis() as _);

            self.cursor_buffer
                .map_mut(0, 0, data.width, data.height, |map| {
                    map.buffer_mut()
                        .chunks_exact_mut(self.cursor_width as usize * 4)
                        .zip(data.pixels_rgba.chunks_exact(data.width as usize * 4))
                        .for_each(|(dst, src)| dst[..src.len()].copy_from_slice(src));
                })?;

            self.cursor_hotspot = (data.xhot as _, data.yhot as _);

            #[allow(deprecated)]
            if self
                .gpu
                .set_cursor2(
                    self.output.crtc,
                    Some(&self.cursor_buffer),
                    self.cursor_hotspot,
                )
                .is_err()
            {
                self.gpu
                    .set_cursor(self.output.crtc, Some(&self.cursor_buffer))?;
            }
        }

        self.output
            .surface
            .swap_buffers(&self.output.context)
            .unwrap();

        let bo = unsafe { self.output.bo.lock_front_buffer().unwrap() };

        let fb = if let Some(handle) = &self.output.fb {
            *handle
        } else {
            let fb = self.gpu.add_framebuffer(&bo, 24, 32).unwrap();
            self.output.fb = Some(fb);
            fb
        };

        if !self.output.crtc_set {
            self.output.crtc_set = true;

            self.gpu.set_crtc(
                self.output.crtc,
                Some(fb),
                (0, 0),
                &[self.output.conn],
                Some(self.output.mode),
            )?;
            self.gpu.page_flip(
                self.output.crtc,
                fb,
                drm::control::PageFlipFlags::empty(),
                None,
            )?;
        } else {
            self.gpu.page_flip(
                self.output.crtc,
                fb,
                drm::control::PageFlipFlags::empty(),
                None,
            )?;
        }

        #[allow(deprecated)]
        if self
            .gpu
            .set_cursor2(
                self.output.crtc,
                Some(&self.cursor_buffer),
                self.cursor_hotspot,
            )
            .is_err()
        {
            self.gpu
                .set_cursor(self.output.crtc, Some(&self.cursor_buffer))?;
        }

        // let mut req = drm::control::atomic::AtomicModeReq::new();
        // req.add_property(output.bo, property, value);
        // self.gpu.atomic_commit(
        //     drm::control::AtomicCommitFlags::PAGE_FLIP_EVENT
        //     | drm::control::AtomicCommitFlags::NONBLOCK,
        //     req,
        // )?;

        Ok(())
    }
}

struct InputState {
    mouse_pos: Pos2,
    events: Vec<egui::Event>,
    key_modifiers: egui::Modifiers,
}

fn evdev_keycode_to_char(code: evdev::KeyCode, shift: bool) -> Option<char> {
    use evdev::KeyCode;

    Some(match code {
        KeyCode::KEY_0 if !shift => '0',
        KeyCode::KEY_1 if !shift => '1',
        KeyCode::KEY_2 if !shift => '2',
        KeyCode::KEY_3 if !shift => '3',
        KeyCode::KEY_4 if !shift => '4',
        KeyCode::KEY_5 if !shift => '5',
        KeyCode::KEY_6 if !shift => '6',
        KeyCode::KEY_7 if !shift => '7',
        KeyCode::KEY_8 if !shift => '8',
        KeyCode::KEY_9 if !shift => '9',

        KeyCode::KEY_0 if shift => ')',
        KeyCode::KEY_1 if shift => '!',
        KeyCode::KEY_2 if shift => '@',
        KeyCode::KEY_3 if shift => '#',
        KeyCode::KEY_4 if shift => '$',
        KeyCode::KEY_5 if shift => '%',
        KeyCode::KEY_6 if shift => '^',
        KeyCode::KEY_7 if shift => '&',
        KeyCode::KEY_8 if shift => '*',
        KeyCode::KEY_9 if shift => '(',

        KeyCode::KEY_GRAVE if !shift => '`',
        KeyCode::KEY_GRAVE if shift => '~',
        KeyCode::KEY_BACKSLASH if !shift => '\\',
        KeyCode::KEY_BACKSLASH if shift => '|',
        KeyCode::KEY_MINUS if !shift => '-',
        KeyCode::KEY_MINUS if shift => '_',
        KeyCode::KEY_EQUAL if !shift => '=',
        KeyCode::KEY_EQUAL if shift => '+',
        KeyCode::KEY_LEFTBRACE if !shift => '[',
        KeyCode::KEY_LEFTBRACE if shift => '{',
        KeyCode::KEY_RIGHTBRACE if !shift => ']',
        KeyCode::KEY_RIGHTBRACE if shift => '}',
        KeyCode::KEY_SEMICOLON if !shift => ';',
        KeyCode::KEY_SEMICOLON if shift => ':',
        KeyCode::KEY_APOSTROPHE if !shift => '\'',
        KeyCode::KEY_APOSTROPHE if shift => '\"',
        KeyCode::KEY_COMMA if !shift => ',',
        KeyCode::KEY_COMMA if shift => '<',
        KeyCode::KEY_DOT if !shift => '.',
        KeyCode::KEY_DOT if shift => '>',
        KeyCode::KEY_SLASH if !shift => '/',
        KeyCode::KEY_SLASH if shift => '?',

        KeyCode::KEY_SPACE => ' ',

        other => {
            let letter = match other {
                KeyCode::KEY_A => 'a',
                KeyCode::KEY_B => 'b',
                KeyCode::KEY_C => 'c',
                KeyCode::KEY_D => 'd',
                KeyCode::KEY_E => 'e',
                KeyCode::KEY_F => 'f',
                KeyCode::KEY_G => 'g',
                KeyCode::KEY_H => 'h',
                KeyCode::KEY_I => 'i',
                KeyCode::KEY_J => 'j',
                KeyCode::KEY_K => 'k',
                KeyCode::KEY_L => 'l',
                KeyCode::KEY_M => 'm',
                KeyCode::KEY_N => 'n',
                KeyCode::KEY_O => 'o',
                KeyCode::KEY_P => 'p',
                KeyCode::KEY_Q => 'q',
                KeyCode::KEY_R => 'r',
                KeyCode::KEY_S => 's',
                KeyCode::KEY_T => 't',
                KeyCode::KEY_U => 'u',
                KeyCode::KEY_V => 'v',
                KeyCode::KEY_W => 'w',
                KeyCode::KEY_X => 'x',
                KeyCode::KEY_Y => 'y',
                KeyCode::KEY_Z => 'z',
                _ => None?,
            };
            if shift {
                letter.to_ascii_uppercase()
            } else {
                letter
            }
        }
    })
}

fn evdev_keycode_to_egui_key(code: evdev::KeyCode) -> Option<egui::Key> {
    use {egui::Key, evdev::KeyCode};
    Some(match code {
        KeyCode::KEY_LEFT => Key::ArrowLeft,
        KeyCode::KEY_RIGHT => Key::ArrowRight,
        KeyCode::KEY_UP => Key::ArrowUp,
        KeyCode::KEY_DOWN => Key::ArrowDown,

        KeyCode::KEY_PAGEUP => Key::PageUp,
        KeyCode::KEY_PAGEDOWN => Key::PageDown,

        KeyCode::KEY_SPACE => Key::Space,
        KeyCode::KEY_TAB => Key::Tab,
        KeyCode::KEY_ENTER => Key::Enter,
        KeyCode::KEY_BACKSPACE => Key::Backspace,
        KeyCode::KEY_DELETE => Key::Delete,
        KeyCode::KEY_ESC => Key::Escape,

        KeyCode::KEY_0 => Key::Num0,
        KeyCode::KEY_1 => Key::Num1,
        KeyCode::KEY_2 => Key::Num2,
        KeyCode::KEY_3 => Key::Num3,
        KeyCode::KEY_4 => Key::Num4,
        KeyCode::KEY_5 => Key::Num5,
        KeyCode::KEY_6 => Key::Num6,
        KeyCode::KEY_7 => Key::Num7,
        KeyCode::KEY_8 => Key::Num8,
        KeyCode::KEY_9 => Key::Num9,

        KeyCode::KEY_A => Key::A,
        KeyCode::KEY_B => Key::B,
        KeyCode::KEY_C => Key::C,
        KeyCode::KEY_D => Key::D,
        KeyCode::KEY_E => Key::E,
        KeyCode::KEY_F => Key::F,
        KeyCode::KEY_G => Key::G,
        KeyCode::KEY_H => Key::H,
        KeyCode::KEY_I => Key::I,
        KeyCode::KEY_J => Key::J,
        KeyCode::KEY_K => Key::K,
        KeyCode::KEY_L => Key::L,
        KeyCode::KEY_M => Key::M,
        KeyCode::KEY_N => Key::N,
        KeyCode::KEY_O => Key::O,
        KeyCode::KEY_P => Key::P,
        KeyCode::KEY_Q => Key::Q,
        KeyCode::KEY_R => Key::R,
        KeyCode::KEY_S => Key::S,
        KeyCode::KEY_T => Key::T,
        KeyCode::KEY_U => Key::U,
        KeyCode::KEY_V => Key::V,
        KeyCode::KEY_W => Key::W,
        KeyCode::KEY_X => Key::X,
        KeyCode::KEY_Y => Key::Y,
        KeyCode::KEY_Z => Key::Z,

        KeyCode::KEY_GRAVE => Key::Backtick,
        KeyCode::KEY_BACKSLASH => Key::Backslash,
        KeyCode::KEY_MINUS => Key::Minus,
        KeyCode::KEY_EQUAL => Key::Equals,
        KeyCode::KEY_LEFTBRACE => Key::OpenBracket,
        KeyCode::KEY_RIGHTBRACE => Key::CloseBracket,
        KeyCode::KEY_SEMICOLON => Key::Semicolon,
        KeyCode::KEY_APOSTROPHE => Key::Quote,
        KeyCode::KEY_COMMA => Key::Comma,
        KeyCode::KEY_DOT => Key::Period,
        KeyCode::KEY_SLASH => Key::Slash,

        _ => None?,
    })
}



struct EventLoop<'a, D> {
    poll: EventPoll,
    event_buffer: Vec<Event>,
    sources: Vec<Option<Box<dyn AnyEventSource<D> + 'a>>>,
}

const MAX_EVENTS_PER_TICK: usize = 8;

impl<'a, D> EventLoop<'a, D> {
    fn new() -> Result<Self> {
        Ok(Self {
            poll: EventPoll::create()?,
            event_buffer: Vec::with_capacity(MAX_EVENTS_PER_TICK),
            sources: Vec::new(),
        })
    }

    fn add_source<S, F>(&mut self, mut source: S, callback: F) -> Result<()>
    where
        S: EventSource<D> + 'a,
        F: FnMut(&mut D, S::Event) -> Result<()> + 'a,
    {
        if let Some(vacant_id) = self.sources.iter().position(|s| s.is_none()) {
            let data = vacant_id as u64;

            source.init(&self.poll, data)?;

            self.sources[vacant_id] = Some(Box::new((source, callback)));
        } else {
            let data = self.sources.len() as u64;

            source.init(&self.poll, data)?;

            self.sources.push(Some(Box::new((source, callback))));
        }

        Ok(())
    }

    fn poll(&mut self, timeout: i32) -> Result<Vec<Event>, kernel::Error> {
        let _event_count = self.poll.wait(&mut self.event_buffer, timeout)?;

        Ok(self.event_buffer.drain(..).collect())
    }

    fn run<F>(mut self, data: &mut D, mut timeout: i32, mut func: F) -> Result<()>
    where
        F: FnMut(&mut D),
    {
        'main_loop: loop {
            let now = Instant::now();

            let events = 'poll_for_events: loop {
                match self.poll(timeout) {
                    Ok(events) => {
                        break 'poll_for_events events;
                    }
                    // If the poll was interrupted, retry until the timeout expires.
                    Err(error) if error == kernel::Error::INTR => {
                        let total_polling_time = now.elapsed().as_millis() as i32;

                        if total_polling_time >= timeout {
                            continue 'main_loop;
                        } else {
                            // Subtract the total polling time from the timeout so the kernel polls
                            // for *exactly* the amount of time requested.
                            timeout -= total_polling_time;
                        }
                    }
                    Err(error) => return Err(error.into()),
                }
            };

            'drain_events: for event in events {
                let response = {
                    let Some(source) = self.sources.get_mut(event.data() as usize) else {
                        continue 'drain_events;
                    };

                    let Some(source) = source else {
                        warn!("Received an event for a nonexistent event source: {event:?}");

                        continue 'drain_events;
                    };

                    source.handle_event(data, event)?
                };

                match response {
                    EventResponse::Continue => {}
                    EventResponse::RemoveSource => {
                        let Some(mut source) = self
                            .sources
                            .get_mut(event.data() as usize)
                            .and_then(|s| s.take())
                        else {
                            // SAFETY: We wouldn't receive `EventResponse::RemoveSource` if it
                            //         didn't already exist.
                            unreachable!()
                        };

                        // ???: Should we just ignore the error here?
                        source.cleanup(&self.poll)?;
                    }
                }
            }

            func(data);
        }
    }
}

pub trait EventSource<D> {
    type Event;

    fn init(&mut self, poll: &EventPoll, key: u64) -> Result<()>;

    fn handle_event<F>(&mut self, data: &mut D, event: Event, callback: F) -> Result<EventResponse>
    where
        F: FnMut(&mut D, Self::Event) -> Result<()>;

    fn cleanup(&mut self, poll: &EventPoll) -> Result<()>;
}

trait AnyEventSource<D> {
    fn handle_event(&mut self, data: &mut D, event: Event) -> Result<EventResponse>;

    fn cleanup(&mut self, poll: &EventPoll) -> Result<()>;
}

impl<D, S, E, F> AnyEventSource<D> for (S, F)
where
    S: EventSource<D, Event = E>,
    F: FnMut(&mut D, E) -> Result<()>,
{
    fn handle_event(&mut self, data: &mut D, event: Event) -> Result<EventResponse> {
        <S as EventSource<D>>::handle_event(&mut self.0, data, event, &mut self.1)
    }

    fn cleanup(&mut self, poll: &EventPoll) -> Result<()> {
        <S as EventSource<D>>::cleanup(&mut self.0, poll)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EventResponse {
    Continue,
    RemoveSource,
}



#[derive(Clone, Debug)]
struct GraphicsCard(Arc<gbm::Device<std::fs::File>>);

impl std::os::unix::io::AsFd for GraphicsCard {
    fn as_fd(&self) -> std::os::unix::io::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl std::ops::Deref for GraphicsCard {
    type Target = gbm::Device<std::fs::File>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Device for GraphicsCard {}

impl ControlDevice for GraphicsCard {}

impl GraphicsCard {
    fn open(path: &str) -> Result<Self> {
        Ok(GraphicsCard(Arc::new(gbm::Device::new(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)?,
        )?)))
    }

    fn debug_info(&self, path: &str) {
        let name = path
            .rsplit_once('/')
            .map_or(path, |(_, name)| name)
            .to_string();

        let driver = match self.get_driver() {
            Ok(driver) => driver,
            Err(error) => {
                error!(target: "gpu_info", "Failed to get driver for {name}: {error}");

                return;
            }
        };

        trace!(
            target: "gpu_info",
            "Graphics driver for {name}:\n\
            \t.name = {}\n\
            \t.date = {}\n\
            \t.description = {}",
            driver.name().display(),
            driver.date().display(),
            driver.description().display(),
        );

        let resources = match self.resource_handles() {
            Ok(resources) => resources,
            Err(error) => {
                error!(target: "gpu_info", "Failed to get resources for {name}: {error}");

                return;
            }
        };

        let planes = match self.plane_handles() {
            Ok(planes) => planes,
            Err(error) => {
                error!(target: "gpu_info", "Failed to get planes for {name}: {error}");

                return;
            }
        };

        let mut found_planes = Vec::new();

        'crtc_iter: for crtc in resources.crtcs() {
            let Ok(info) = self.get_crtc(*crtc) else {
                warn!(
                    target: "gpu_info",
                    "Failed to get CRTC info for {name} at {crtc:?}",
                );

                continue 'crtc_iter;
            };

            trace!(
                target: "gpu_info",
                "{}\n\
                \t.position: ({}, {})\n\
                \t.mode: {}\n\
                \t.framebuffer: {:?}\n\
                \t.gamma_length: {}",
                info,
                info.position().0, info.position().1,
                if let Some(mode) = info.mode() {
                    format!(
                        "{}\n\
                        \t\t.size: ({}, {})\n\
                        \t\t.clock_speed: {} kHz\n\
                        \t\t.vrefresh: {}\n\
                        \t\t.flags: {:?}\n\
                        \t\t.mode_type: {:?}",
                        mode.name().to_string_lossy(),
                        mode.size().0, mode.size().1,
                        mode.clock(),
                        mode.vrefresh(),
                        mode.flags(),
                        mode.mode_type(),
                    )
                } else {
                    "NONE".to_string()
                },
                info.framebuffer(),
                info.gamma_length(),
            );

            for plane in &planes {
                let Ok(plane_info) = self.get_plane(*plane) else {
                    warn!(target: "gpu_info", "Failed to get plane info for {name} at {plane:?}");

                    continue;
                };

                if plane_info.crtc() != Some(*crtc) {
                    continue;
                }

                found_planes.push(*plane);

                trace!(
                    target: "gpu_info",
                    "Plane for {crtc:?}: {plane:?}\n\
                    \t.fb = {:?}\n\
                    \t.formats = {:?}",
                    plane_info.framebuffer(),
                    plane_info.formats(),
                );

                if let Some(fb) = plane_info.framebuffer() {
                    let Ok(fb_info) = self.get_planar_framebuffer(fb) else {
                        trace!(target: "gpu_info", "Info unavailable for {fb:?}");

                        continue;
                    };

                    trace!(
                        target: "gpu_info",
                        "PLANAR_FRAMEBUFFER for {plane:?} @{fb:?}:\n\
                        \t.buffers = {:?}\n\
                        \t.flags = {:?}\n\
                        \t.modifier = {:?}\n\
                        \t.pixel_format = {:?}",
                        fb_info.buffers(),
                        fb_info.flags(),
                        fb_info.modifier(),
                        fb_info.pixel_format(),
                    );
                }
            }

            if let Ok(properties) = self.get_properties(*crtc) {
                'prop_iter: for (prop, raw_value) in properties {
                    let Ok(info) = self.get_property(prop) else {
                        warn!(
                            target: "gpu_info",
                            "Failed to get property info for {name} at {prop:?}",
                        );

                        continue 'prop_iter;
                    };

                    trace!(
                        target: "gpu_info",
                        "Property for {crtc:?}: {} = {:?}",
                        info.name().to_string_lossy(),
                        info.value_type().convert_value(raw_value),
                    );
                }
            }
        }

        for plane in planes {
            if !found_planes.contains(&plane) {
                let Ok(plane_info) = self.get_plane(plane) else {
                    trace!(target: "gpu_info", "Info unavailable for {plane:?}");

                    continue;
                };

                trace!(
                    target: "gpu_info",
                    "PLANE @{:?} (DISCONNECTED):\n\
                    \t.crtc = {:?}\n\
                    \t.possible_crtcs = {:?}\n\
                    \t.fb = {:?}\n\
                    \t.formats = {:?}",
                    plane,
                    plane_info.crtc(),
                    plane_info.possible_crtcs(),
                    plane_info.framebuffer(),
                    plane_info.formats(),
                );
            }

            if let Ok(properties) = self.get_properties(plane) {
                'prop_iter: for (prop, raw_value) in properties {
                    let Ok(info) = self.get_property(prop) else {
                        warn!(
                            target: "gpu_info",
                            "Failed to get property info for {plane:?} at {prop:?}",
                        );

                        continue 'prop_iter;
                    };

                    trace!(
                        target: "gpu_info",
                        "Property for {plane:?}: {} = {:?}",
                        info.name().to_string_lossy(),
                        info.value_type().convert_value(raw_value),
                    );
                }
            }
        }
    }

    fn prepare_output(
        &self,
        config: &glutin::api::egl::config::Config,
        context: glutin::api::egl::context::NotCurrentContext,
    ) -> Result<Output> {
        let resources = self.resource_handles()?;

        for conn in resources.connectors().iter().copied() {
            let conn_info = self.get_connector(conn, true)?;

            let Some(enc) = conn_info.current_encoder() else {
                continue;
            };

            let enc_info = self.get_encoder(enc)?;

            let Some(crtc) = enc_info.crtc() else {
                continue;
            };

            let Some(mode) = conn_info.modes().iter().find(|mode| {
                mode.mode_type()
                    .contains(drm::control::ModeTypeFlags::PREFERRED)
            }) else {
                continue;
            };

            let bo = self.create_surface(
                mode.size().0 as _,
                mode.size().1 as _,
                gbm::Format::Argb8888,
                gbm::BufferObjectFlags::SCANOUT | gbm::BufferObjectFlags::RENDERING,
            )?;

            let surface = unsafe {
                context
                    .display()
                    .create_window_surface(
                        &config,
                        &glutin::surface::SurfaceAttributesBuilder::<
                            glutin::surface::WindowSurface
                        >::new()
                            .build(
                                raw_window_handle::RawWindowHandle::Gbm(
                                    raw_window_handle::GbmWindowHandle::new(
                                        NonNull::new(bo.as_raw() as *mut _).unwrap()
                                    ),
                                ),
                                NonZeroU32::new(mode.size().0 as _).unwrap(),
                                NonZeroU32::new(mode.size().1 as _).unwrap(),
                            ))
                    .unwrap()
            };

            let context = context.make_current(&surface)?;

            surface.set_swap_interval(
                &context,
                glutin::surface::SwapInterval::Wait(NonZeroU32::MIN),
            )?;

            let renderer = egl::Renderer::new(&context.display())?;

            return Ok(Output {
                bo,
                fb: None,
                conn,
                crtc,
                mode: *mode,
                renderer,
                surface,
                context,
                crtc_set: false,
            });
        }

        bail!("no valid outputs found")
    }
}

impl EventSource<Shell> for GraphicsCard {
    type Event = drm::control::Event;

    fn init(&mut self, poll: &EventPoll, key: u64) -> Result<()> {
        poll.add(
            &unsafe { File::from_raw(self.as_raw_fd()) },
            Event::new(key, true, false),
        )?;

        Ok(())
    }

    fn handle_event<F>(
        &mut self,
        data: &mut Shell,
        _event: Event,
        mut callback: F,
    ) -> Result<EventResponse>
    where
        F: FnMut(&mut Shell, Self::Event) -> Result<()>,
    {
        for event in self.receive_events()? {
            callback(data, event)?;
        }

        Ok(EventResponse::Continue)
    }

    fn cleanup(&mut self, poll: &EventPoll) -> Result<()> {
        poll.remove(&unsafe { File::from_raw(self.as_raw_fd()) })?;

        Ok(())
    }
}



struct Output {
    bo: gbm::Surface<drm::control::framebuffer::Handle>,
    fb: Option<drm::control::framebuffer::Handle>,
    conn: drm::control::connector::Handle,
    crtc: drm::control::crtc::Handle,
    mode: drm::control::Mode,
    renderer: egl::Renderer,
    surface: glutin::api::egl::surface::Surface<glutin::surface::WindowSurface>,
    context: glutin::api::egl::context::PossiblyCurrentContext,
    crtc_set: bool,
}

impl Output {
    pub fn width(&self) -> u16 {
        self.mode.size().0
    }

    pub fn height(&self) -> u16 {
        self.mode.size().1
    }
}



#[unsafe(export_name = "__shell_info")]
#[allow(non_snake_case)]
extern "Rust" fn __shell_info(text: &str) {
    info!(target: "extern", "{text}")
}
