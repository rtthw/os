
pub mod egl;
pub mod input;
pub mod log;
pub mod object;

use std::{
    ffi::OsString,
    io::{BufRead as _, Read as _, Write as _},
    os::fd::AsRawFd as _,
    str::FromStr as _, sync::Arc, time::Instant
};

use anyhow::{Result, bail};
use drm::{Device, control::Device as ControlDevice};
use kernel::epoll::{Event, EventPoll};
use ::log::{debug, error, info, trace, warn};

use crate::object::Object;



fn main() -> Result<()> {
    log::Logger::default().init()?;

    info!("Starting shell...");
    std::thread::sleep(std::time::Duration::from_secs(1));

    let gpu = GraphicsCard::open("/dev/dri/card0")?;

    egl::init().expect("failed to initialize EGL");

    let egl_extensions = egl::extensions().expect("failed to get EGL extensions");
    trace!(target: "gpu", "Supported EGL client extensions: {:?}", egl_extensions);

    trace!(target: "gpu", "Initializing EGL...");

    let gbm = gbm::Device::new(gpu.clone()).expect("failed to create GBM device");
    let display = egl::Display::new(&gbm).expect("failed to initialize EGL display");
    let device = egl::Device::for_display(&display).expect("failed to get EGL device");

    let egl_dpy_extensions = device.extensions().expect("failed to get EGL display extensions");
    trace!(target: "gpu", "Supported EGL display extensions: {:?}", egl_dpy_extensions);

    let egl_dev_extensions = device.extensions().expect("failed to get EGL device extensions");
    trace!(target: "gpu", "Supported EGL device extensions: {:?}", egl_dev_extensions);

    if egl_dev_extensions.iter().any(|e| e == "EGL_MESA_device_software") {
        panic!("No render node available");
    }

    let context = egl::Context::new(&display).expect("failed to initialize EGL context");

    unsafe {
        context.make_current().unwrap();
    }

    gpu.set_client_capability(drm::ClientCapability::UniversalPlanes, true)
        .expect("unable to request gpu.UniversalPlanes capability");
    gpu.set_client_capability(drm::ClientCapability::Atomic, true)
        .expect("unable to request gpu.Atomic capability");

    trace!(target: "gpu", "Preparing outputs...");

    let mut clear_color: [u8; 4] = [51, 43, 43, 255];
    let mut outputs = match gpu.prepare_outputs(clear_color) {
        Ok(outputs) => outputs,
        Err(error) => {
            bail!(
                "\x1b[31mERROR\x1b[0m \x1b[2m(shell)\x1b[0m: \
                Failed to prepare outputs: {error}",
            );
        }
    };

    gpu.debug_info("/dev/dri/card0");

    for output in &outputs {
        gpu.set_crtc(
            output.crtc,
            Some(output.fb),
            (0, 0),
            &[output.conn],
            Some(output.mode),
        ).unwrap();
    }

    let this_obj = unsafe { Object::open_this().expect("should be able to open shell binary") };

    let stdin = std::io::stdin();
    unsafe {
        assert_ne!(libc::fcntl(stdin.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK), -1);
    }

    let mut event_loop = EventLoop::new()?;

    for (path, device) in evdev::enumerate() {
        let name = device.name().unwrap_or("Unnamed Device").to_string();
        debug!(
            target: "dev",
            "{}\n\
            \t.name: {}\n\
            \t.physical_path: {}\n\
            \t.properties: {:?}\n\
            \t.supported_events: {:?}\n\
            \t.supported_keys: {:?}",
            path.display(),
            &name,
            device.physical_path().unwrap_or("NONE"),
            device.properties(),
            device.supported_events(),
            device.supported_keys(),
        );

        event_loop.add_source(input::InputSource::new(device)?, move |_shell, input_event| {
            trace!(target: "input", "EVENT @ {name}: {input_event:?}");
            Ok(())
        })?;
    }

    let mut shell = Shell {
        current_dir: std::env::current_dir().unwrap().to_str().unwrap().to_string(),
    };

    print!("\x1b[2m {} }} \x1b[0m", &shell.current_dir);

    std::io::stdout().flush().unwrap();

    event_loop.run(&mut shell, 10, |shell| {
        if stdin.lock().read(&mut []).is_err() {
            return;
        }

        let mut line = String::new();
        let Ok(_) = stdin.lock().take(256).read_line(&mut line) else { return; };
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
                    shell.current_dir = std::env::current_dir().unwrap()
                        .to_str().unwrap()
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
            "ls" => {
                let mut names = Vec::new();
                for entry in std::fs::read_dir(&shell.current_dir).unwrap() {
                    let entry = entry.unwrap();
                    let name = entry.path().file_name().unwrap().to_str().unwrap().to_string();
                    if name.contains(' ') {
                        names.push(format!("'{name}'"));
                    } else {
                        names.push(name);
                    }
                }
                println!("{}", names.join("  "));
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
            "clear" => 'handle_clear: {
                if args.len() >= 4 {
                    let Ok(r) = u8::from_str_radix(args[1], 10) else {
                        println!("Invalid red channel: {}", args[1]);
                        break 'handle_clear;
                    };
                    let Ok(g) = u8::from_str_radix(args[2], 10) else {
                        println!("Invalid green channel: {}", args[2]);
                        break 'handle_clear;
                    };
                    let Ok(b) = u8::from_str_radix(args[3], 10) else {
                        println!("Invalid blue channel: {}", args[3]);
                        break 'handle_clear;
                    };
                    let a = {
                        if args.len() == 5 {
                            let Ok(a) = u8::from_str_radix(args[4], 10) else {
                                println!("Invalid alpha channel: {}", args[4]);
                                break 'handle_clear;
                            };
                            a
                        } else {
                            255
                        }
                    };

                    clear_color = [b, g, r, a];

                    'map_outputs: for output in &mut outputs {
                        let Ok(mut map) = gpu.map_dumb_buffer(&mut output.db) else {
                            println!(
                                "\x1b[31mERROR\x1b[0m \x1b[2m(shell.clear)\x1b[0m: \
                                Failed to map output buffer",
                            );
                            continue 'map_outputs;
                        };
                        for pixel in map.chunks_exact_mut(4) {
                            pixel[0] = clear_color[0];
                            pixel[1] = clear_color[1];
                            pixel[2] = clear_color[2];
                            pixel[3] = clear_color[3];
                        }
                        if let Err(error) = gpu.set_crtc(
                            output.crtc,
                            Some(output.fb),
                            (0, 0),
                            &[output.conn],
                            Some(output.mode),
                        ) {
                            println!(
                                "\x1b[31mERROR\x1b[0m \x1b[2m(shell.clear)\x1b[0m: \
                                Failed to set CRTC: {error}",
                            );
                        }
                    }
                } else {
                    println!("Invalid arguments");
                }
            }
            _ => {
                let bin_path = format!("/bin/{}", args[0]);
                match std::process::Command::new(bin_path).args(&args_os[1..]).output() {
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
    current_dir: String,
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
                        let Some(mut source) = self.sources
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
struct GraphicsCard(Arc<std::fs::File>);

impl std::os::unix::io::AsFd for GraphicsCard {
    fn as_fd(&self) -> std::os::unix::io::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Device for GraphicsCard {}

impl ControlDevice for GraphicsCard {}

impl GraphicsCard {
    fn open(path: &str) -> Result<Self> {
        Ok(GraphicsCard(Arc::new(std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?)))
    }

    fn debug_info(&self, path: &str) {
        let name = path.rsplit_once('/').map_or(path, |(_, name)| name).to_string();

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
                    \t.fb = {:?}\n\
                    \t.formats = {:?}",
                    plane,
                    plane_info.framebuffer(),
                    plane_info.formats(),
                );
            }
        }
    }

    fn prepare_outputs(&self, clear_color: [u8; 4]) -> Result<Vec<Output>> {
        let mut outputs = Vec::with_capacity(1);

        let resources = self.resource_handles()?;
        for conn in resources.connectors().iter().copied() {
            let conn_info = self.get_connector(conn, true)?;
            let Some(enc) = conn_info.current_encoder() else { continue; };
            let enc_info = self.get_encoder(enc)?;
            let Some(crtc) = enc_info.crtc() else { continue; };
            let crtc_info = self.get_crtc(crtc)?;
            let Some(mode) = crtc_info.mode() else { continue; };

            let mut db = self.create_dumb_buffer(
                (mode.size().0 as _, mode.size().1 as _),
                drm::buffer::DrmFourcc::Xrgb8888,
                32,
            )?;

            {
                let mut map = self.map_dumb_buffer(&mut db)?;
                for pixel in map.chunks_exact_mut(4) {
                    pixel[0] = clear_color[0];
                    pixel[1] = clear_color[1];
                    pixel[2] = clear_color[2];
                    pixel[3] = clear_color[3];
                }
            }

            let fb = self.add_framebuffer(&db, 24, 32)?;

            outputs.push(Output { db, fb, conn, crtc, mode });
        }

        Ok(outputs)
    }
}

struct Output {
    db: drm::control::dumbbuffer::DumbBuffer,
    fb: drm::control::framebuffer::Handle,
    conn: drm::control::connector::Handle,
    crtc: drm::control::crtc::Handle,
    mode: drm::control::Mode,
}
