
use crate::{file::File, proc::Process};



pub trait AsFile {
    fn as_file(&self) -> File;
}

pub trait AsProcess {
    fn as_process(&self) -> Process;
}
