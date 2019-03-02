use log::*;

#[test]
fn you_must_not_see_anything() {
    flexi_logger::Logger::with_str("info")
        .do_not_log()
        .start()
        .unwrap();

    error!("This is an error message - you must not see it!");
    warn!("This is a warning - you must not see it!");
    info!("This is an info message - you must not see it!");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}
