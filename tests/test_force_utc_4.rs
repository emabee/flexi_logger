use flexi_logger::{detailed_format, test_utils, FileSpec, Logger};
use jiff::{
    civil::{Date, DateTime, Time},
    ZonedDateTime,
};
use log::*;

#[test]
fn test_force_utc_4() {
    let mut path = test_utils::dir();

    let _ = Logger::try_with_str("info")
        .unwrap()
        .use_utc()
        .format(detailed_format)
        .log_to_file(
            FileSpec::default()
                .directory(&path)
                .basename("test")
                .suppress_timestamp(),
        )
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    info!("must be printed");
    let now = ZonedDateTime::now();

    // parse timestamp from written file
    path.push("test.log");
    let s = std::fs::read_to_string(path).unwrap();
    let d = DateTime::from_parts(
        s[1..11].parse::<Date>().unwrap(),
        s[12..27].parse::<Time>().unwrap(),
    );

    if now.offset().seconds().abs() > 100 {
        // local TZ is different from UTC -> verify that UTC was written to the file
        let now_local = now.datetime();
        let diff = now_local.duration_since(d).as_secs();
        println!("d: {d}, now_local: {now_local}, diff: {diff}");
        assert!(diff.abs() >= 10);
    }
}
