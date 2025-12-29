
use std::{ffi::OsString, io::{BufRead as _, Read as _, Write as _}, str::FromStr as _};



fn main() {
    println!("\x1b[2mshell\x1b[0m: Starting...");
    std::thread::sleep(std::time::Duration::from_secs(1));

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
                        println!("{}", error.to_string());
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
                _ => {
                    match std::process::Command::new(&args_os[0]).args(&args_os[1..]).output() {
                        Ok(output) => {
                            println!("{}", String::from_utf8(output.stdout).unwrap());
                        }
                        Err(error) => {
                            println!("{}", error.to_string());
                        }
                    }
                }
            }

            std::io::stdout().flush().unwrap();
        }
    }
}
