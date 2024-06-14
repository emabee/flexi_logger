mod test_utils;

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use flexi_logger::{detailed_format, FileSpec, Logger};
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
    let now = Local::now();

    // parse timestamp from written file
    path.push("test.log");
    let s = std::fs::read_to_string(path).unwrap();
    let d: NaiveDateTime = NaiveDateTime::new(
        s[1..11].parse::<NaiveDate>().unwrap(),
        s[12..27].parse::<NaiveTime>().unwrap(),
    );

    if now.offset().utc_minus_local().abs() > 100 {
        // local TZ is different from UTC -> verify that UTC was written to the file
        let now_local = now.naive_local();
        let diff = (now_local - d).num_seconds();
        println!("d: {d}, now_local: {now_local}, diff: {diff}");
        assert!(diff.abs() >= 10);
    }
}
