use flexi_logger::writers::LogWriter;
use std::{
    io::Error,
    sync::{Arc, Mutex},
};

fn main() {}

#[allow(dead_code)]
struct MyWriter<W> {
    writer: Arc<Mutex<W>>,
}

impl<F: std::io::Write + Send + Sync> LogWriter for MyWriter<F> {
    fn write(
        &self,
        now: &mut flexi_logger::DeferredNow,
        record: &flexi_logger::Record,
    ) -> std::io::Result<()> {
        let mut file = self
            .writer
            .lock()
            .map_err(|e| Error::other(e.to_string()))?;
        flexi_logger::default_format(&mut *file, now, record)
    }

    fn flush(&self) -> std::io::Result<()> {
        let mut file = self
            .writer
            .lock()
            .map_err(|e| Error::other(e.to_string()))?;
        file.flush()
    }
}
