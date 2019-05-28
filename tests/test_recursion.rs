use flexi_logger::{detailed_format, Logger};
use log::*;

#[test]
fn test_recursion() {
    Logger::with_str("info")
        .format(detailed_format)
        .log_to_file()
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed because: {}", e));

    let foo = Foo();

    for _ in 0..10 {
        error!("This is an error message for {}", foo);
        warn!("This is a warning for {}", foo);
        info!("This is an info message for {}", foo);
        debug!("This is a debug message for {}", foo);
        trace!("This is a trace message for {}", foo);
    }
}

struct Foo();
impl std::fmt::Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        info!("Here comes the inner message :-| ");
        f.write_str("Foo!!")?;
        Ok(())
    }
}
