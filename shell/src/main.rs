
use std::{ffi::OsString, io::{BufRead as _, Read as _, Write as _}, str::FromStr as _};



fn main() {
    println!("\x1b[2mshell\x1b[0m: Starting...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    let stdin = std::io::stdin();
    loop {
        print!("\x1b[2m}} \x1b[0m");
        std::io::stdout().flush().unwrap();
        let mut line = String::new();
        if let Ok(_bytes_read) = stdin.lock().take(256).read_line(&mut line) {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            let args: Vec<OsString> = line
                .split(' ')
                .map(|item| OsString::from_str(item).unwrap())
                .collect();
            let exe = args.get(0).unwrap();
            if exe == "exit" {
                std::process::exit(0);
            }
            match std::process::Command::new(exe).args(&args[1..]).output() {
                Ok(output) => {
                    println!("{}", String::from_utf8(output.stdout).unwrap());
                }
                Err(error) => {
                    println!("{}", error.to_string());
                }
            }
            std::io::stdout().flush().unwrap();
        }
    }
}
