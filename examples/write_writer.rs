#![allow(dead_code)]
use flexi_logger::writers::LogWriter;
use std::{
    io::{Error, ErrorKind},
    sync::{Arc, Mutex},
};

fn main() {}

struct MyWriter<F> {
    file: Arc<Mutex<F>>,
}

impl<F: std::io::Write + Send + Sync> LogWriter for MyWriter<F> {
    fn write(
        &self,
        now: &mut flexi_logger::DeferredNow,
        record: &flexi_logger::Record,
    ) -> std::io::Result<()> {
        let mut file = self
            .file
            .lock()
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        flexi_logger::default_format(&mut *file, now, record)
    }

    fn flush(&self) -> std::io::Result<()> {
        let mut file = self
            .file
            .lock()
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        file.flush()
    }
}
