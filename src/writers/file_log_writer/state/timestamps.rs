use super::{get_creation_timestamp, InfixFilter, InfixFormat};
use crate::{writers::FileLogWriterConfig, FileSpec};
use chrono::{format::ParseErrorKind, DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use std::path::{Path, PathBuf};

pub(super) fn infix_from_timestamp(
    ts: &DateTime<Local>,
    use_utc: bool,
    fmt: &InfixFormat,
) -> String {
    if use_utc {
        ts.naive_utc().format(fmt.format())
    } else {
        ts.format(fmt.format())
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
) -> Result<DateTime<Local>, String> {
    match NaiveDateTime::parse_from_str(infix, fmt.format()) {
        Ok(dt1) => Local
            .from_local_datetime(&dt1)
            .earliest()
            .ok_or("Can't determine local time from infix".to_string()),
        Err(e) if e.kind() == ParseErrorKind::NotEnough => {
            match NaiveDate::parse_from_str(infix, fmt.format()) {
                Ok(d1) => {
                    Local
                        .from_local_datetime(&d1.and_hms_opt(10, 0, 0).unwrap(/*OK*/))
                        .earliest()
                        .ok_or("Can't determine local time from infix".to_string())
                }
                Err(e) => Err(format!("Broken: {e:?}")),
            }
        }
        Err(e) => Err(format!("Broken: {e:?}")),
    }
}

pub(super) fn creation_timestamp_of_currentfile(
    config: &FileLogWriterConfig,
    current_infix: &str,
    rotate_rcurrent: bool,
    o_date_for_rotated_file: Option<&DateTime<Local>>,
    fmt: &InfixFormat,
) -> Result<DateTime<Local>, std::io::Error> {
    let current_path = config.file_spec.as_pathbuf(Some(current_infix));

    if rotate_rcurrent {
        let date_for_rotated_file = o_date_for_rotated_file
            .copied()
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
) -> DateTime<Local> {
    if rotate {
        Local::now()
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
            // if nothing is found, take Local::now()
            .unwrap_or_else(Local::now)
    }
}

fn path_for_rotated_file_from_timestamp(
    file_spec: &FileSpec,
    use_utc: bool,
    timestamp_for_rotated_file: &DateTime<Local>,
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
    use chrono::{Duration, Local};
    use std::path::PathBuf;

    #[test]
    fn test_latest_timestamp_file() {
        let file_spec = FileSpec::default()
            .basename("basename")
            .directory("direc/tory")
            .discriminant("disc")
            .suppress_timestamp();

        let now = Local::now();
        let now_rounded = now
            .checked_sub_signed(
                Duration::from_std(std::time::Duration::from_nanos(u64::from(
                    now.timestamp_subsec_nanos(),
                )))
                .unwrap(),
            )
            .unwrap();

        let paths: Vec<PathBuf> = (0..10)
            .map(|i| now_rounded - Duration::try_seconds(i).unwrap())
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
            // if nothing is found, take Local::now()
            .unwrap_or_else(Local::now);

        assert_eq!(
            now_rounded,
            // TODO: use mocking to avoid code duplication:
            // this test is only useful if the path evaluation is the same as in
            // super::latest_timestamp_file()
            newest
        );
    }
}
