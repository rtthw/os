
pub mod egl;
pub mod input;
pub mod object;

use std::{
    ffi::OsString,
    io::{BufRead as _, Read as _, Write as _},
    os::fd::AsRawFd as _,
    str::FromStr as _, sync::Arc
};

use anyhow::{Result, bail};
use drm::{Device, control::Device as ControlDevice};
use kernel::epoll::{Event, EventPoll};

use crate::object::Object;



fn main() -> Result<()> {
    println!("\x1b[2mshell\x1b[0m: Starting...");
    std::thread::sleep(std::time::Duration::from_secs(1));

    let gpu = GraphicsCard::open("/dev/dri/card0")?;

    egl::init().expect("failed to initialize EGL");

    let egl_extensions = egl::extensions().expect("failed to get EGL extensions");
    println!("\x1b[2mshell.gpu\x1b[0m: Supported EGL client extensions: {:?}", egl_extensions);

    println!("\x1b[2mshell.gpu\x1b[0m: Initializing EGL...");

    let gbm = gbm::Device::new(gpu.clone()).expect("failed to create GBM device");
    let display = egl::Display::new(&gbm).expect("failed to initialize EGL display");
    let device = egl::Device::for_display(&display).expect("failed to get EGL device");

    let egl_dpy_extensions = device.extensions().expect("failed to get EGL display extensions");
    println!("\x1b[2mshell.gpu\x1b[0m: Supported EGL display extensions: {:?}", egl_dpy_extensions);

    let egl_dev_extensions = device.extensions().expect("failed to get EGL device extensions");
    println!("\x1b[2mshell.gpu\x1b[0m: Supported EGL device extensions: {:?}", egl_dev_extensions);

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

    println!("\x1b[2mshell.gpu\x1b[0m: Preparing outputs...");

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

    gpu.print_info("/dev/dri/card0");

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
        println!("\x1b[2mshell.dev\x1b[0m: {}", path.display());
        println!("\t.name: {}", &name);
        println!("\t.physical_path: {}", device.physical_path().unwrap_or("NONE"));
        println!("\t.properties: {:?}", device.properties());
        println!("\t.supported_events: {:?}", device.supported_events());
        println!("\t.supported_keys: {:?}", device.supported_keys());

        event_loop.add_source(input::InputSource::new(device)?, move |_shell, input_event| {
            println!("INPUT @ {name}: {input_event:?}");
            Ok(())
        })?;
    }

    let mut shell = Shell {};

    let current_dir = std::env::current_dir().unwrap();
    print!("\x1b[2m {} }} \x1b[0m", current_dir.display());

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
                let current_dir = std::env::current_dir().unwrap();
                let mut names = Vec::new();
                for entry in std::fs::read_dir(&current_dir).unwrap() {
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

        let current_dir = std::env::current_dir().unwrap();
        print!("\x1b[2m {} }} \x1b[0m", current_dir.display());

        std::io::stdout().flush().unwrap();
    })
}



pub struct Shell {}



struct EventLoop<'a, D> {
    poll: EventPoll,
    event_buffer: Vec<Event>,
    sources: Vec<Box<dyn AnyEventSource<D> + 'a>>,
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
        let data = self.sources.len() as u64;
        source.init(&self.poll, data)?;
        self.sources.push(Box::new((source, callback)));
        Ok(())
    }

    fn poll(&mut self, timeout: i32) -> Result<Vec<Event>> {
        let _event_count = self.poll.wait(&mut self.event_buffer, timeout)?;
        Ok(self.event_buffer.drain(..).collect())
    }

    fn run<F>(mut self, data: &mut D, timeout: i32, mut func: F) -> Result<()>
    where
        F: FnMut(&mut D),
    {
        loop {
            'drain_events: for event in self.poll(timeout)? {
                let Some(source) = self.sources.get_mut(event.data() as usize) else {
                    continue 'drain_events;
                };
                source.handle_event(data, event)?;
            }
            func(data);
        }
    }
}

pub trait EventSource<D> {
    type Event;

    fn init(&mut self, poll: &EventPoll, key: u64) -> Result<()>;
    fn handle_event<F>(&mut self, data: &mut D, event: Event, callback: F) -> Result<()>
    where
        F: FnMut(&mut D, Self::Event) -> Result<()>;
}

trait AnyEventSource<D> {
    fn handle_event(&mut self, data: &mut D, event: Event) -> Result<()>;
}

impl<D, S, E, F> AnyEventSource<D> for (S, F)
where
    S: EventSource<D, Event = E>,
    F: FnMut(&mut D, E) -> Result<()>,
{
    fn handle_event(&mut self, data: &mut D, event: Event) -> Result<()> {
        <S as EventSource<D>>::handle_event(&mut self.0, data, event, &mut self.1)
    }
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

    fn print_info(&self, path: &str) {
        let name = path.rsplit_once('/').map_or(path, |(_, name)| name).to_string();

        let driver = match self.get_driver() {
            Ok(driver) => driver,
            Err(error) => {
                println!(
                    "\x1b[31mERROR\x1b[0m \x1b[2m(shell.gpu_info.{name})\x1b[0m: \
                    Failed to get driver: {error}",
                );
                return;
            }
        };

        println!("\x1b[2mshell.gpu_info.{name}\x1b[0m: DRIVER:");

        println!("\t.name = {}", driver.name().display());
        println!("\t.date = {}", driver.date().display());
        println!("\t.description = {}", driver.description().display());

        let resources = match self.resource_handles() {
            Ok(resources) => resources,
            Err(error) => {
                println!(
                    "\x1b[31mERROR\x1b[0m \x1b[2m(shell.gpu_info.{name})\x1b[0m: \
                    Failed to get resources: {error}",
                );
                return;
            }
        };
        let planes = match self.plane_handles() {
            Ok(planes) => planes,
            Err(error) => {
                println!(
                    "\x1b[31mERROR\x1b[0m \x1b[2m(shell.gpu_info.{name})\x1b[0m: \
                    Failed to get planes: {error}",
                );
                return;
            }
        };

        let mut found_planes = Vec::new();
        'crtc_iter: for crtc in resources.crtcs() {
            let Ok(info) = self.get_crtc(*crtc) else {
                println!(
                    "\x1b[33mWARN\x1b[0m \x1b[2m(shell.gpu_info.{name})\x1b[0m: \
                    Failed to get CRTC info at {:?}",
                    crtc,
                );
                continue 'crtc_iter;
            };

            println!("\x1b[2mshell.gpu_info.{name}\x1b[0m: CRTC:");

            println!("\t.position = {},{}", info.position().0, info.position().1);

            if let Some(mode) = info.mode() {
                println!("\t.size = {},{}", mode.size().0, mode.size().1);
                println!("\t.clock_speed = {} kHz", mode.clock());
            } else {
                println!("\t.mode = NONE");
                return;
            };

            for plane in &planes {
                let Ok(plane_info) = self.get_plane(*plane) else {
                    println!(
                        "\x1b[33mWARN\x1b[0m \x1b[2m(shell.gpu_info.{name})\x1b[0m: \
                        Failed to get plane info at {:?}",
                        plane,
                    );
                    continue;
                };
                if plane_info.crtc() != Some(*crtc) {
                    continue;
                }
                found_planes.push(*plane);
                println!("\tPLANE @{:?}:", plane);
                println!("\t\t.fb = {:?}", plane_info.framebuffer());
                println!("\t\t.formats = {:?}", plane_info.formats());
            }

            if let Ok(properties) = self.get_properties(*crtc) {
                'prop_iter: for (prop, raw_value) in properties {
                    let Ok(info) = self.get_property(prop) else {
                        println!(
                            "\x1b[33mWARN\x1b[0m \x1b[2m(shell.gpu_info.{name})\x1b[0m: \
                            Failed to get property info at {:?}",
                            prop,
                        );
                        continue 'prop_iter;
                    };

                    println!(
                        "\t...: {} = {:?}",
                        info.name().to_string_lossy(),
                        info.value_type().convert_value(raw_value),
                    );
                }
            }
        }

        for plane in planes {
            if !found_planes.contains(&plane) {
                println!(
                    "\x1b[2mshell.gpu_info.{name}\x1b[0m: PLANE @{:?} (DISCONNECTED):",
                    plane,
                );
                let Ok(plane_info) = self.get_plane(plane) else {
                    println!("\tINFO UNAVAILABLE");
                    continue;
                };
                println!("\t.fb = {:?}", plane_info.framebuffer());
                println!("\t.formats = {:?}", plane_info.formats());
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
