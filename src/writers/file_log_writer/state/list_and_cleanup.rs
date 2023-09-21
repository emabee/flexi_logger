use crate::{Cleanup, FileSpec, LogfileSelector};
#[cfg(feature = "compress")]
use std::fs::File;
use std::{
    path::PathBuf,
    thread::{Builder as ThreadBuilder, JoinHandle},
};

const INFIX_PATTERN: &str = "_r[0-9]*";

pub(super) fn existing_log_files(file_spec: &FileSpec, selector: &LogfileSelector) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if selector.with_plain_files {
        let pattern = file_spec.as_glob_pattern(INFIX_PATTERN, file_spec.get_suffix().as_deref());
        result.append(&mut list_of_files(&pattern));
    }

    if selector.with_compressed_files {
        let pattern = file_spec.as_glob_pattern(INFIX_PATTERN, Some("gz"));
        result.append(&mut list_of_files(&pattern));
    }

    if selector.with_r_current {
        let pattern =
            file_spec.as_glob_pattern(super::CURRENT_INFIX, file_spec.get_suffix().as_deref());
        result.append(&mut list_of_files(&pattern));
    }
    result
}

pub(super) fn list_of_log_and_compressed_files(file_spec: &FileSpec) -> Vec<PathBuf> {
    existing_log_files(
        file_spec,
        &LogfileSelector::default().with_compressed_files(),
    )
}

pub(super) fn list_of_infix_files() -> Vec<PathBuf> {
    list_of_files(INFIX_PATTERN)
}
fn list_of_files(pattern: &str) -> Vec<PathBuf> {
    let mut log_files: Vec<PathBuf> = glob::glob(pattern)
        .unwrap(/* failure should be impossible */)
        .filter_map(Result::ok)
        .collect();
    log_files.sort_unstable(); // should be no-op, but we don't want to rely on glob doing it
    log_files.reverse();
    log_files
}

pub(super) fn remove_or_compress_too_old_logfiles(
    o_cleanup_thread_handle: &Option<CleanupThreadHandle>,
    cleanup_config: &Cleanup,
    file_spec: &FileSpec,
) -> Result<(), std::io::Error> {
    o_cleanup_thread_handle.as_ref().map_or_else(
        || remove_or_compress_too_old_logfiles_impl(cleanup_config, file_spec),
        |cleanup_thread_handle| {
            cleanup_thread_handle
                .sender
                .send(MessageToCleanupThread::Act)
                .ok();
            Ok(())
        },
    )
}

pub(crate) fn remove_or_compress_too_old_logfiles_impl(
    cleanup_config: &Cleanup,
    file_spec: &FileSpec,
) -> Result<(), std::io::Error> {
    let (log_limit, compress_limit) = match *cleanup_config {
        Cleanup::Never => {
            return Ok(());
        }
        Cleanup::KeepLogFiles(log_limit) => (log_limit, 0),

        #[cfg(feature = "compress")]
        Cleanup::KeepCompressedFiles(compress_limit) => (0, compress_limit),

        #[cfg(feature = "compress")]
        Cleanup::KeepLogAndCompressedFiles(log_limit, compress_limit) => {
            (log_limit, compress_limit)
        }
    };

    for (index, file) in list_of_log_and_compressed_files(file_spec)
        .into_iter()
        .enumerate()
    {
        if index >= log_limit + compress_limit {
            // delete (log or log.gz)
            std::fs::remove_file(file)?;
        } else if index >= log_limit {
            #[cfg(feature = "compress")]
            {
                // compress, if not yet compressed
                if let Some(extension) = file.extension() {
                    if extension != "gz" {
                        let mut old_file = File::open(file.clone())?;
                        let mut compressed_file = file.clone();
                        compressed_file.set_extension("log.gz");
                        let mut gz_encoder = flate2::write::GzEncoder::new(
                            File::create(compressed_file)?,
                            flate2::Compression::fast(),
                        );
                        std::io::copy(&mut old_file, &mut gz_encoder)?;
                        gz_encoder.finish()?;
                        std::fs::remove_file(&file)?;
                    }
                }
            }
        }
    }

    Ok(())
}

const CLEANER: &str = "flexi_logger-fs-cleanup";

#[derive(Debug)]
pub(super) struct CleanupThreadHandle {
    sender: std::sync::mpsc::Sender<MessageToCleanupThread>,
    join_handle: JoinHandle<()>,
}

enum MessageToCleanupThread {
    Act,
    Die,
}
impl CleanupThreadHandle {
    pub(super) fn shutdown(self) {
        self.sender.send(MessageToCleanupThread::Die).ok();
        self.join_handle.join().ok();
    }
}

pub(super) fn start_cleanup_thread(
    cleanup: Cleanup,
    file_spec: FileSpec,
) -> Result<CleanupThreadHandle, std::io::Error> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let builder = ThreadBuilder::new().name(CLEANER.to_string());
    #[cfg(not(feature = "dont_minimize_extra_stacks"))]
    let builder = builder.stack_size(512 * 1024);
    Ok(CleanupThreadHandle {
        sender,
        join_handle: builder.spawn(move || {
            while let Ok(MessageToCleanupThread::Act) = receiver.recv() {
                remove_or_compress_too_old_logfiles_impl(&cleanup, &file_spec).ok();
            }
        })?,
    })
}
