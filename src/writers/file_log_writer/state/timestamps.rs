use super::{get_creation_timestamp, list_and_cleanup::list_of_infix_files, InfixFormat};
use crate::{writers::FileLogWriterConfig, FileSpec};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use std::path::Path;
use std::{ops::Add, path::PathBuf};

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
        .as_pathbuf(Some("_rXXXXX"))
        .to_string_lossy()
        .find("_rXXXXX")
        .unwrap();
    String::from_utf8_lossy(&path.to_string_lossy().as_bytes()[idx..idx + 21]).to_string()
}
fn timestamp_from_ts_infix(infix: &str, fmt: &InfixFormat) -> Option<DateTime<Local>> {
    NaiveDateTime::parse_from_str(infix, fmt.format())
        .ok()
        .and_then(|ts| Local.from_local_datetime(&ts).single())
}

pub(super) fn rcurrents_creation_timestamp(
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
        list_of_infix_files()
            .into_iter()
            // retrieve the infix
            .map(|path| ts_infix_from_path(&path, &config.file_spec))
            // parse infix as date, ignore all infixes where this fails
            .filter_map(|infix| timestamp_from_ts_infix(&infix, fmt))
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
    let infix = collision_free_infix_for_rotated_file(
        file_spec,
        &infix_from_timestamp(timestamp_for_rotated_file, use_utc, fmt),
    );
    file_spec.as_pathbuf(Some(&infix))
}

// handles collisions by appending ".restart-<number>" to the infix, if necessary
pub(super) fn collision_free_infix_for_rotated_file(file_spec: &FileSpec, infix: &str) -> String {
    let mut new_path = file_spec.as_pathbuf(Some(infix));
    let mut new_path_with_gz = new_path.clone();
    match new_path.extension() {
        Some(oss) => {
            let mut oss_gz = oss.to_os_string();
            oss_gz.push(".gz");
            new_path_with_gz.set_extension(oss_gz.as_os_str());
        }
        None => {
            new_path_with_gz.set_extension("gz");
        }
    }

    // search for restart-siblings
    let mut pattern = new_path.clone();
    if file_spec.o_suffix.is_some() {
        pattern.set_extension("");
    }
    let mut pattern = pattern.to_string_lossy().to_string();
    pattern.push_str(".restart-*");
    let mut restart_siblings = glob::glob(&pattern)
        .unwrap(/* PatternError should be impossible */)
        // ignore all files with GlobError
        .filter_map(Result::ok)
        .collect::<Vec<PathBuf>>();

    // if collision would occur (new_path or compressed new_path exists already),
    // find highest restart and add 1, else continue without restart
    if new_path.exists() || new_path_with_gz.exists() || !restart_siblings.is_empty() {
        let next_number = if restart_siblings.is_empty() {
            0
        } else {
            restart_siblings.sort_unstable();
            new_path = restart_siblings.pop().unwrap(/*ok*/);
            let file_stem_string = if file_spec.o_suffix.is_some() {
                new_path
                    .file_stem().unwrap(/*ok*/)
                    .to_string_lossy().to_string()
            } else {
                new_path.to_string_lossy().to_string()
            };
            let index = file_stem_string.find(".restart-").unwrap(/*ok*/);
            file_stem_string[(index + 9)..(index + 13)].parse::<usize>().unwrap(/*ok*/) + 1
        };

        infix.to_string().add(&format!(".restart-{next_number:04}"))
    } else {
        infix.to_string()
    }
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
        let now = now
            .checked_sub_signed(
                Duration::from_std(std::time::Duration::from_nanos(u64::from(
                    now.timestamp_subsec_nanos(),
                )))
                .unwrap(),
            )
            .unwrap();
        let paths: Vec<PathBuf> = (0..10)
            .map(|i| now - Duration::try_seconds(i).unwrap())
            .map(|ts| {
                file_spec.as_pathbuf(Some(&super::infix_from_timestamp(
                    &ts,
                    false,
                    &InfixFormat::Std,
                )))
            })
            .collect();

        assert_eq!(
            now,
            // TODO: use mocking to avoid code duplication:
            // this test is only useful if the path evaluation is the same as in
            // super::latest_timestamp_file()
            paths
                .iter()
                // retrieve the infix
                .map(|path| super::ts_infix_from_path(path, &file_spec))
                // parse infix as date, ignore all files where this fails,
                .filter_map(|infix| super::timestamp_from_ts_infix(&infix, &InfixFormat::Std))
                // take the newest of these dates
                .reduce(|acc, e| if acc > e { acc } else { e })
                // if nothing is found, take Local::now()
                .unwrap_or_else(Local::now)
        );
    }
}
