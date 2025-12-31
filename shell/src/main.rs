
pub mod object;

use std::{ffi::OsString, io::{BufRead as _, Read as _, Write as _}, str::FromStr as _};

use drm::{Device, control::Device as ControlDevice};

use crate::object::Object;



fn main() {
    println!("\x1b[2mshell\x1b[0m: Starting...");
    std::thread::sleep(std::time::Duration::from_secs(1));

    print_gpu_info("/dev/dri/card0");

    let this_obj = unsafe { Object::open_this().expect("should be able to open shell binary") };

    let stdin = std::io::stdin();
    loop {
        let current_dir = std::env::current_dir().unwrap();
        print!("\x1b[2m {} }} \x1b[0m", current_dir.display());

        std::io::stdout().flush().unwrap();

        let mut line = String::new();
        if let Ok(_bytes_read) = stdin.lock().take(256).read_line(&mut line) {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
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

            std::io::stdout().flush().unwrap();
        }
    }
}



fn print_gpu_info(path: &str) {
    let gpu = GraphicsCard::open(path);
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name).to_string();

    let driver = match gpu.get_driver() {
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

    let resources = match gpu.resource_handles() {
        Ok(resources) => resources,
        Err(error) => {
            println!(
                "\x1b[31mERROR\x1b[0m \x1b[2m(shell.gpu_info.{name})\x1b[0m: \
                Failed to get resources: {error}",
            );
            return;
        }
    };

    'crtc_iter: for crtc in resources.crtcs() {
        let Ok(info) = gpu.get_crtc(*crtc) else {
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

        if let Ok(properties) = gpu.get_properties(*crtc) {
            'prop_iter: for (prop, raw_value) in properties {
                let Ok(info) = gpu.get_property(prop) else {
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
}



#[derive(Debug)]
struct GraphicsCard(std::fs::File);

impl std::os::unix::io::AsFd for GraphicsCard {
    fn as_fd(&self) -> std::os::unix::io::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Device for GraphicsCard {}

impl ControlDevice for GraphicsCard {}

impl GraphicsCard {
    fn open(path: &str) -> Self {
        GraphicsCard(std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap())
    }
}
