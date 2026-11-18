use super::{get_creation_timestamp, InfixFilter, InfixFormat};
use crate::{writers::FileLogWriterConfig, FileSpec};
use jiff::{fmt::strtime::BrokenDownTime, tz::TimeZone, ZonedDateTime};
use std::path::{Path, PathBuf};

pub(super) fn infix_from_timestamp(ts: &ZonedDateTime, use_utc: bool, fmt: &InfixFormat) -> String {
    if use_utc {
        ts.with_time_zone(TimeZone::UTC).strftime(fmt.format())
    } else {
        ts.strftime(fmt.format())
    }
    .to_string()
}

fn ts_infix_from_path(path: &Path, file_spec: &FileSpec) -> String {
    let idx = file_spec
        .as_pathbuf(Some("rXXXXX"))
        .to_string_lossy()
        .find("rXXXXX")
        .unwrap();
    String::from_utf8_lossy(&path.to_string_lossy().as_bytes()[idx..idx + 20]).to_string()
}

pub(crate) fn timestamp_from_ts_infix(
    infix: &str,
    fmt: &InfixFormat,
) -> Result<ZonedDateTime, String> {
    let mut bdt = BrokenDownTime::parse(fmt.format(), infix)
        .map_err(|e| format!("Parsing infix failed: {e:?}"))?;
    if bdt.hour().is_none() {
        // see https://github.com/emabee/flexi_logger/issues/181
        bdt.set_hour(Some(10)).unwrap(/*Ok*/);
    }
    let dt = bdt
        .to_datetime()
        .map_err(|e| format!("Evaluating infix failed: {e:?}"))?;

    TimeZone::system()
        .to_zoned_datetime(dt)
        .map_err(|e| format!("Can't determine local time from infix: {e:?}"))
}

pub(super) fn creation_timestamp_of_currentfile(
    config: &FileLogWriterConfig,
    current_infix: &str,
    rotate_rcurrent: bool,
    o_date_for_rotated_file: Option<&ZonedDateTime>,
    fmt: &InfixFormat,
) -> Result<ZonedDateTime, std::io::Error> {
    let current_path = config.file_spec.as_pathbuf(Some(current_infix));

    if rotate_rcurrent {
        let date_for_rotated_file = o_date_for_rotated_file
            .cloned()
            .unwrap_or_else(|| get_creation_timestamp(&current_path));
        let rotated_path = path_for_rotated_file_from_timestamp(
            &config.file_spec,
            config.use_utc,
            &date_for_rotated_file,
            fmt,
        );

        match std::fs::rename(current_path.clone(), rotated_path.clone()) {
            Ok(()) => {}
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    return Err(e);
                }
            }
        }
    }
    Ok(get_creation_timestamp(&current_path))
}

// determine the timestamp to which we want to write (file needn't exist)
pub(super) fn latest_timestamp_file(
    config: &FileLogWriterConfig,
    rotate: bool,
    fmt: &InfixFormat,
) -> ZonedDateTime {
    if rotate {
        ZonedDateTime::now()
    } else {
        // find all file paths that fit the pattern
        config
            .file_spec
            .list_of_files(
                &InfixFilter::Numbrs,
                config.file_spec.get_suffix().as_deref(),
            )
            .into_iter()
            // retrieve the infix
            .map(|path| ts_infix_from_path(&path, &config.file_spec))
            // parse infix as date, ignore all infixes where this fails
            .filter_map(|infix| timestamp_from_ts_infix(&infix, fmt).ok())
            // take the newest of these dates
            .reduce(|acc, e| if acc > e { acc } else { e })
            // if nothing is found, take ZonedDateTime::now()
            .unwrap_or_else(ZonedDateTime::now)
    }
}

fn path_for_rotated_file_from_timestamp(
    file_spec: &FileSpec,
    use_utc: bool,
    timestamp_for_rotated_file: &ZonedDateTime,
    fmt: &InfixFormat,
) -> PathBuf {
    let infix = file_spec.collision_free_infix_for_rotated_file(&infix_from_timestamp(
        timestamp_for_rotated_file,
        use_utc,
        fmt,
    ));
    file_spec.as_pathbuf(Some(&infix))
}

#[cfg(test)]
mod test {
    use super::InfixFormat;
    use crate::FileSpec;
    use jiff::{SignedDuration, ZonedDateTime};
    use std::path::PathBuf;

    #[test]
    fn test_latest_timestamp_file() {
        let file_spec = FileSpec::default()
            .basename("basename")
            .directory("direc/tory")
            .discriminant("disc")
            .suppress_timestamp();

        let now = ZonedDateTime::now();
        let now_rounded = now
            .checked_sub(SignedDuration::from_nanos(i64::from(
                now.subsec_nanosecond(),
            )))
            .unwrap();

        let paths: Vec<PathBuf> = (0..10)
            .map(|i| {
                now_rounded
                    .checked_sub(SignedDuration::from_secs(i))
                    .unwrap()
            })
            .map(|ts| {
                file_spec.as_pathbuf(Some(&super::infix_from_timestamp(
                    &ts,
                    false,
                    &InfixFormat::Std,
                )))
            })
            .collect();

        let newest = paths
            .iter()
            // retrieve the infix
            .map(|path| super::ts_infix_from_path(path, &file_spec))
            // parse infix as date, ignore all files where this fails,
            .filter_map(|infix| super::timestamp_from_ts_infix(&infix, &InfixFormat::Std).ok())
            // take the newest of these dates
            .reduce(|acc, e| if acc > e { acc } else { e })
            // if nothing is found, take ZonedDateTime::now()
            .unwrap_or_else(ZonedDateTime::now);

        assert_eq!(
            now_rounded,
            // TODO: use mocking to avoid code duplication:
            // this test is only useful if the path evaluation is the same as in
            // super::latest_timestamp_file()
            newest
        );
    }
}
