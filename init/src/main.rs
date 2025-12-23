//! # Init System

use kernel::{proc::Process, raw::exit};



fn main() {
    if Process::current() != 1 {
        exit(-1)
    }

    loop {
        match update() {
            Ok(after_update) => {
                match after_update {
                    AfterUpdate::Exit => exit(0),
                }
            }
            Err(exit_status) => exit(exit_status),
        }
    }
}

fn update() -> Result<AfterUpdate, i32> {
    let mut after_update = None;
    while after_update.is_none() {
        after_update = Some(AfterUpdate::Exit);
    }

    Ok(after_update.unwrap())
}

enum AfterUpdate {
    Exit,
}
