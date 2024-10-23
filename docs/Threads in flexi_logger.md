# Threads in `flexi_logger`

## src/threads.rs

### "flexi_logger-flusher"

* called in Logger::build if NOT WriteMode::Direct, WriteMode::SupportCapture,
  WriteMode::BufferDontFlush or WriteMode::BufferDontFlushWith(_) is chosen
* pub(crate) fn start_flusher_thread(
* flushes primary writer and other writers with flush_interval cadence
* stack_size(1024)

### "flexi_logger-async_std_writer"

* only available with feature "async"
* called in constructor of StdWriter if WriteMode::Async or WriteMode::AsyncWith is chosen
* [cfg(feature = "async")] pub(crate) fn start_async_stdwriter(
* flushes, or writes to stdout or stderr
* rust default stack_size = 2 \* 1024 \* 1024

## src/writers/file_log_writer/state.rs

### FLW: "flexi_logger-async_file_writer"

* only available with feature "async"
* Called in intialization of the FLW, if WriteMode::Async or WriteMode::AsyncWith is chosen
* [cfg(feature = "async")] pub(super) fn start_async_fs_writer(
* flushes, or writes to the FLW's buffer
* rust default stack_size = 2 \* 1024 \* 1024

### FLW: "flexi_logger-file_flusher"

* ONLY USED if FLW is used in custom LogWriter implementation
* Called in intialization of the FLW, if WriteMode::Direct, WriteMode::SupportCapture,
  WriteMode::BufferDontFlush or WriteMode::BufferDontFlushWith(_) is chosen and if flush_interval > 0
  Note that flexi_logger sets flush_interval = 0 for its "embedded" FLW!
* pub(super) fn start_sync_flusher(
* flushes the FLW's file
* stack_size(1024)

### "flexi_logger-fs-async_flusher"

* only available with feature "async"
* Called in intialization of the FLW if WriteMode::Async/With and if flush_interval > 0
* pub(crate) fn start_async_fs_flusher(
* triggers the flush on the "flexi_logger-async_file_writer"
* stack_size(1024)

## src/writers/file_log_writer/state/list_and_cleanup.rs

### "flexi_logger-fs-cleanup"

* only called when explicitly configured
* pub(super) fn start_cleanup_thread(
* calls remove_or_compress_too_old_logfiles_impl
* stack_size(512 * 1024)
