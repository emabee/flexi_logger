use crate::{writers::file_log_writer::InfixFilter, DeferredNow, FlexiLoggerError};
use std::{
    ffi::{OsStr, OsString},
    ops::Add,
    path::{Path, PathBuf},
};

/// Builder object for specifying the name and path of the log output file.
///
/// The filename is built from several partially components, using this pattern:
///
/// ```<filename> = [<basename>][_][<discriminant>][_][<starttime>][_][<infix>][.<suffix>]```
///
/// - `[<basename>]`: This is by default the program's name, but can be set to a different value
///   or suppressed at all.
///
/// - `[_]`: Consecutive name parts are separated by an underscore.
///   No underscore is used at the beginning of the filename and directly before the suffix.
///
/// - `[<discriminant>]`: some optional name part that allows further differentiations.
///
/// - `[<starttime>]`: denotes the point in time when the program was started, if used.
///
/// - `[infix]`: used with rotation to differentiate consecutive files.
///
/// Without rotation, the default filename pattern uses the program name as basename,
/// no discriminant, the timestamp of the program start
/// (printed in the format "YYYY-MM-DD_hh-mm-ss"),
/// and the suffix `.log`, e.g.
///
/// ```myprog_2015-07-08_10-44-11.log```.
///
/// This ensures that with every program start a new trace file is written that can easily
/// be associated with a concrete program run.
///
/// When the timestamp is suppressed with [`FileSpec::suppress_timestamp`],
/// you get a fixed output file name.
/// It is then worth considering whether a new program start should discard
/// the content of an already existing outputfile or if it should append its new content to it
/// (see [`Logger::append`](crate::Logger::append)).
///
/// With rotation, the timestamp is by default suppressed and instead the infix is used.
/// The infix starts always with "r".
/// For more details how its precise content can be influenced, see [`Naming`](crate::Naming).
///
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileSpec {
    pub(crate) directory: PathBuf,
    pub(crate) basename: String,
    pub(crate) o_discriminant: Option<String>,
    timestamp_cfg: TimestampCfg,
    o_suffix: Option<String>,
    pub(crate) use_utc: bool,
}
impl Default for FileSpec {
    /// Describes a file in the current folder,
    /// using, as its filestem, the program name followed by the current timestamp,
    /// and the suffix ".log".
    fn default() -> Self {
        FileSpec {
            directory: PathBuf::from("."),
            basename: Self::default_basename(),
            o_discriminant: None,
            timestamp_cfg: TimestampCfg::Default,
            o_suffix: Some(String::from("log")),
            use_utc: false,
        }
    }
}
impl FileSpec {
    fn default_basename() -> String {
        let arg0 = std::env::args().next().unwrap_or_else(|| "rs".to_owned());
        Path::new(&arg0).file_stem().map(OsStr::to_string_lossy).unwrap(/*cannot fail*/).to_string()
    }

    /// The provided path should describe a log file.
    /// If it exists, it must be a file, not a folder.
    /// If necessary, parent folders will be created.
    ///
    /// ```rust
    /// # use flexi_logger::FileSpec;
    /// assert_eq!(
    ///     FileSpec::default()
    ///         .directory("/a/b/c")
    ///         .basename("foo")
    ///         .suppress_timestamp()
    ///         .suffix("bar"),
    ///     FileSpec::try_from("/a/b/c/foo.bar").unwrap()
    /// );
    /// ```
    /// # Errors
    ///
    /// [`FlexiLoggerError::OutputBadFile`] if the given path exists and is a folder.
    /// [`FlexiLoggerError::BadFileSpec`] if deriving the `FileSpec` from the given path fails.
    #[allow(clippy::missing_panics_doc)]
    pub fn try_from<P: Into<PathBuf>>(p: P) -> Result<Self, FlexiLoggerError> {
        let input: PathBuf = p.into();

        if input.is_dir() {
            Err(FlexiLoggerError::BadFileSpec("File path is a directory"))
        } else {
            let input_as_str = input.as_os_str().to_string_lossy();
            if input_as_str.is_empty() {
                Err(FlexiLoggerError::BadFileSpec("File path is empty"))
            } else if input_as_str.ends_with('/')
                || input_as_str.ends_with("/.")
                || input_as_str.ends_with("/..")
            {
                Err(FlexiLoggerError::BadFileSpec(
                    "Path ends with '/' or '/.' or '/..'",
                ))
            } else if input
                .file_name()
                .ok_or(FlexiLoggerError::OutputBadFile)?
                .to_string_lossy()
                .starts_with('.')
                && input.extension().is_none()
            {
                Err(FlexiLoggerError::BadFileSpec(
                    "File name cannot start with '.' without an extension",
                ))
            } else {
                match input.parent() {
                    None => Err(FlexiLoggerError::BadFileSpec(
                        "File path has no parent directory",
                    )),
                    Some(parent) => {
                        let filespec = FileSpec {
                            directory: if parent.as_os_str().is_empty() {
                                PathBuf::from(".")
                            } else {
                                parent.to_path_buf()
                            },
                            basename: input.file_stem().unwrap(/*ok*/).to_string_lossy().to_string(),
                            o_discriminant: None,
                            o_suffix: input.extension().map(|s| s.to_string_lossy().to_string()),
                            timestamp_cfg: TimestampCfg::No,
                            use_utc: false,
                        };
                        Ok(filespec)
                    }
                }
            }
        }
    }

    /// Makes the logger not include a basename into the names of the log files
    ///
    /// Equivalent to `basename("")`.
    #[must_use]
    pub fn suppress_basename(self) -> Self {
        self.basename("")
    }

    /// The specified String is used as the basename of the log file name,
    /// instead of the program name. Using a file separator within the argument is discouraged.
    #[must_use]
    pub fn basename<S: Into<String>>(mut self, basename: S) -> Self {
        self.basename = basename.into();
        self
    }

    /// The specified String is used as the basename of the log file,
    /// instead of the program name, which is used when `None` is given.
    #[must_use]
    pub fn o_basename<S: Into<String>>(mut self, o_basename: Option<S>) -> Self {
        self.basename = o_basename.map_or_else(Self::default_basename, Into::into);
        self
    }

    /// Specifies a folder for the log files.
    ///
    /// If the specified folder does not exist, it will be created.
    /// By default, the log files are created in the folder where the program was started.
    #[must_use]
    pub fn directory<P: Into<PathBuf>>(mut self, directory: P) -> Self {
        self.directory = directory.into();
        self
    }

    /// Specifies a folder for the log files.
    ///
    /// If the specified folder does not exist, it will be created.
    /// With None, the log files are created in the folder where the program was started.
    #[must_use]
    pub fn o_directory<P: Into<PathBuf>>(mut self, directory: Option<P>) -> Self {
        self.directory = directory.map_or_else(|| PathBuf::from("."), Into::into);
        self
    }

    /// The specified String is added to the log file name.
    #[must_use]
    pub fn discriminant<S: Into<String>>(self, discriminant: S) -> Self {
        self.o_discriminant(Some(discriminant))
    }

    /// The specified String is added to the log file name.
    #[must_use]
    pub fn o_discriminant<S: Into<String>>(mut self, o_discriminant: Option<S>) -> Self {
        self.o_discriminant = o_discriminant.map(Into::into);
        self
    }
    /// Specifies a suffix for the log files.
    ///
    /// Equivalent to `o_suffix(Some(suffix))`.
    #[must_use]
    pub fn suffix<S: Into<String>>(self, suffix: S) -> Self {
        self.o_suffix(Some(suffix))
    }

    /// Specifies a suffix for the log files, or supresses the use of a suffix completely.
    ///
    /// The default suffix is "log".
    #[must_use]
    pub fn o_suffix<S: Into<String>>(mut self, o_suffix: Option<S>) -> Self {
        self.o_suffix = o_suffix.map(Into::into);
        self
    }

    /// Makes the logger not include the start time into the names of the log files
    ///
    /// Equivalent to `use_timestamp(false)`.
    #[must_use]
    pub fn suppress_timestamp(self) -> Self {
        self.use_timestamp(false)
    }

    /// Defines if the start time should be included into the names of the log files.
    ///
    /// The _default_ behavior depends on the usage:
    /// - without rotation, a timestamp is by default included into the name
    /// - with rotation, the timestamp is by default suppressed
    #[must_use]
    pub fn use_timestamp(mut self, use_timestamp: bool) -> Self {
        self.timestamp_cfg = if use_timestamp {
            TimestampCfg::Yes
        } else {
            TimestampCfg::No
        };
        self
    }

    #[doc(hidden)]
    #[must_use]
    pub fn used_directory(&self) -> PathBuf {
        self.directory.clone()
    }
    pub(crate) fn has_basename(&self) -> bool {
        !self.basename.is_empty()
    }
    pub(crate) fn has_discriminant(&self) -> bool {
        self.o_discriminant.is_some()
    }
    pub(crate) fn uses_timestamp(&self) -> bool {
        matches!(self.timestamp_cfg, TimestampCfg::Yes)
    }

    // If no decision was done yet, decide now whether to include a timestamp
    // into the names of the log files.
    pub(crate) fn if_default_use_timestamp(&mut self, use_timestamp: bool) {
        if let TimestampCfg::Default = self.timestamp_cfg {
            self.timestamp_cfg = if use_timestamp {
                TimestampCfg::Yes
            } else {
                TimestampCfg::No
            };
        }
    }

    pub(crate) fn get_directory(&self) -> PathBuf {
        self.directory.clone()
    }

    pub(crate) fn get_suffix(&self) -> Option<String> {
        self.o_suffix.clone()
    }

    // basename + o_discriminant + o_timestamp
    pub(crate) fn fixed_name_part(&self) -> String {
        let mut fixed_name_part = self.basename.clone();
        fixed_name_part.reserve(50);

        if let Some(discriminant) = &self.o_discriminant {
            append_underscore_if_not_empty(&mut fixed_name_part);
            fixed_name_part.push_str(discriminant);
        }
        if let Some(timestamp) = &self.timestamp_cfg.get_timestamp() {
            append_underscore_if_not_empty(&mut fixed_name_part);
            fixed_name_part.push_str(timestamp);
        }
        fixed_name_part
    }

    /// Derives a `PathBuf` from the spec and the given infix.
    #[must_use]
    pub fn as_pathbuf(&self, o_infix: Option<&str>) -> PathBuf {
        let mut filename = self.fixed_name_part();

        if let Some(infix) = o_infix {
            if !infix.is_empty() {
                append_underscore_if_not_empty(&mut filename);
                filename.push_str(infix);
            }
        }
        if let Some(suffix) = &self.o_suffix {
            filename.push('.');
            filename.push_str(suffix);
        }

        let mut p_path = self.directory.clone();
        p_path.push(filename);
        p_path
    }

    // handles collisions by appending ".restart-<number>" to the infix, if necessary
    pub(crate) fn collision_free_infix_for_rotated_file(&self, infix: &str) -> String {
        let uncompressed_files = self.list_of_files(
            &InfixFilter::Equls(infix.to_string()),
            self.o_suffix.as_deref(),
        );
        let compressed_files =
            self.list_of_files(&InfixFilter::Equls(infix.to_string()), Some("gz"));

        let mut restart_siblings = uncompressed_files
            .into_iter()
            .chain(compressed_files)
            .filter(|pb| {
                // ignore .gz suffix
                let mut pb2 = PathBuf::from(pb);
                if pb2.extension() == Some(OsString::from("gz").as_ref()) {
                    pb2.set_extension("");
                }
                // suffix must match the given suffix, if one is given
                match self.o_suffix {
                    Some(ref sfx) => pb2.extension() == Some(OsString::from(sfx).as_ref()),
                    None => true,
                }
            })
            .filter(|pb| {
                pb.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains(".restart-")
            })
            .collect::<Vec<PathBuf>>();

        let new_path = self.as_pathbuf(Some(infix));
        let new_path_with_gz = {
            let mut new_path_with_gz = new_path.clone();
            new_path_with_gz
                .set_extension([self.o_suffix.as_deref().unwrap_or(""), ".gz"].concat());
            new_path_with_gz
        };

        // if collision would occur (new_path or compressed new_path exists already),
        // find highest restart and add 1, else continue without restart
        if new_path.exists() || new_path_with_gz.exists() || !restart_siblings.is_empty() {
            let next_number = if restart_siblings.is_empty() {
                0
            } else {
                restart_siblings.sort_unstable();
                let new_path = restart_siblings.pop().unwrap(/*ok*/);
                let file_stem_string = if self.o_suffix.is_some() {
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

    pub(crate) fn list_of_files(
        &self,
        infix_filter: &InfixFilter,
        o_suffix: Option<&str>,
    ) -> Vec<PathBuf> {
        self.filter_files(&self.read_dir_related_files(), infix_filter, o_suffix)
    }

    // returns an ordered list of all files in the right directory that start with the fixed_name_part
    pub(crate) fn read_dir_related_files(&self) -> Vec<PathBuf> {
        let fixed_name_part = self.fixed_name_part();
        let mut log_files = std::fs::read_dir(&self.directory)
            .unwrap(/*ignore errors from reading the directory*/)
            .flatten(/*ignore errors from reading entries in the directory*/)
            .filter(|entry| entry.path().is_file())
            .map(|de| de.path())
            .filter(|path| {
                // fixed name part must match
                if let Some(fln) = path.file_name() {
                    fln.to_string_lossy(/*good enough*/).starts_with(&fixed_name_part)
                } else {
                    false
                }
            })
            .collect::<Vec<PathBuf>>();
        log_files.sort_unstable();
        log_files.reverse();
        log_files
    }

    pub(crate) fn filter_files(
        &self,
        files: &[PathBuf],
        infix_filter: &InfixFilter,
        o_suffix: Option<&str>,
    ) -> Vec<PathBuf> {
        let fixed_name_part = self.fixed_name_part();
        files
            .iter()
            .filter(|path| {
                // if suffix is specified, it must match
                if let Some(suffix) = o_suffix {
                    path.extension().is_some_and(|ext| {
                        let s = ext.to_string_lossy();
                        s == suffix
                    })
                } else {
                    true
                }
            })
            .filter(|path| {
                // infix filter must pass
                let stem = path.file_stem().unwrap(/* CANNOT FAIL*/).to_string_lossy();
                let infix_start = if fixed_name_part.is_empty() {
                    0
                } else {
                    fixed_name_part.len() + 1 // underscore at the end
                };
                if stem.len() <= infix_start {
                    return false;
                }
                let maybe_infix = &stem[infix_start..];
                let end = maybe_infix.find('.').unwrap_or(maybe_infix.len());
                infix_filter.filter_infix(&maybe_infix[..end])
            })
            .map(PathBuf::clone)
            .collect::<Vec<PathBuf>>()
    }

    #[cfg(test)]
    pub(crate) fn get_timestamp(&self) -> Option<String> {
        self.timestamp_cfg.get_timestamp()
    }
}

fn append_underscore_if_not_empty(filename: &mut String) {
    if !filename.is_empty() {
        filename.push('_');
    }
}

const TS_USCORE_DASHES_USCORE_DASHES: &str = "%Y-%m-%d_%H-%M-%S";

#[derive(Debug, Clone, Eq, PartialEq)]
enum TimestampCfg {
    Default,
    Yes,
    No,
}
impl TimestampCfg {
    fn get_timestamp(&self) -> Option<String> {
        match self {
            Self::Default | Self::Yes => Some(
                DeferredNow::new()
                    .format(TS_USCORE_DASHES_USCORE_DASHES)
                    .to_string(),
            ),
            Self::No => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::{FileSpec, TimestampCfg};
    use crate::writers::file_log_writer::InfixFilter;
    use std::{
        fs::File,
        path::{Path, PathBuf},
    };

    #[test]
    fn test_timstamp_cfg() {
        let ts = TimestampCfg::Yes;
        let s = ts.get_timestamp().unwrap(/* OK */);
        let bytes = s.into_bytes();
        assert_eq!(bytes[4], b'-');
        assert_eq!(bytes[7], b'-');
        assert_eq!(bytes[10], b'_');
        assert_eq!(bytes[13], b'-');
        assert_eq!(bytes[16], b'-');
    }

    #[test]
    fn test_default() {
        let path = FileSpec::default().as_pathbuf(None);
        assert_file_spec(&path, &PathBuf::from("."), true, "log");
    }

    #[test]
    fn issue_194() {
        assert!(dbg!(FileSpec::try_from("")).is_err());
        assert!(dbg!(FileSpec::try_from(".")).is_err());
        assert!(dbg!(FileSpec::try_from("..")).is_err());
        // assert!(dbg!(FileSpec::try_from("/Users")).is_err());
        // assert!(dbg!(FileSpec::try_from("/Users/")).is_err());
        assert!(dbg!(FileSpec::try_from("./f/")).is_err());
        assert!(dbg!(FileSpec::try_from("./f/.")).is_err());
        assert!(dbg!(FileSpec::try_from("./f/..")).is_err());
        assert!(dbg!(FileSpec::try_from(".log")).is_err());
        assert!(dbg!(FileSpec::try_from("./.log")).is_err());
        assert!(dbg!(FileSpec::try_from("./f/.log")).is_err());

        let filespec = FileSpec::try_from("test.log").unwrap();
        std::fs::create_dir_all(filespec.get_directory()).unwrap();
        assert!(std::fs::metadata(filespec.get_directory())
            .unwrap()
            .is_dir());
    }

    // todo: does not support suppress_timestamp & suppress_basename & use discriminant
    fn assert_file_spec(path: &Path, folder: &Path, with_timestamp: bool, suffix: &str) {
        // check folder
        assert_eq!(
            path.parent().unwrap(), // .canonicalize().unwrap()
            folder                  // .canonicalize().unwrap()
        );
        // check file stem
        //  - should start with progname
        let progname = PathBuf::from(std::env::args().next().unwrap())
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .clone()
            .to_string();
        let stem = path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .clone()
            .to_string();
        assert!(
            stem.starts_with(&progname),
            "stem: {stem:?}, progname: {progname:?}",
        );
        if with_timestamp {
            // followed by _ and timestamp
            assert_eq!(stem.as_bytes()[progname.len()], b'_');
            let s_ts = &stem[progname.len() + 1..];
            assert!(
                chrono::NaiveDateTime::parse_from_str(s_ts, "%Y-%m-%d_%H-%M-%S").is_ok(),
                "s_ts: \"{s_ts}\"",
            );
        } else {
            assert_eq!(
                stem.len(),
                progname.len(),
                "stem: {stem:?}, progname: {progname:?}",
            );
        }

        // check suffix
        assert_eq!(path.extension().unwrap(), suffix);
    }

    #[test]
    fn test_if_default_use_timestamp() {
        // default() + if_default_use_timestamp(false) => false
        {
            let mut fs = FileSpec::default();
            fs.if_default_use_timestamp(false);
            let path = fs.as_pathbuf(None);
            assert_file_spec(&path, &PathBuf::from("."), false, "log");
        }
        // default() + use_timestamp(true) + if_default_use_timestamp(false) => true
        {
            let mut fs = FileSpec::default().use_timestamp(true);
            fs.if_default_use_timestamp(false);
            let path = fs.as_pathbuf(None);
            assert_file_spec(&path, &PathBuf::from("."), true, "log");
        }
        // default() + use_timestamp(false) + if_default_use_timestamp(true) +  => true
        {
            let mut fs = FileSpec::default();
            fs.if_default_use_timestamp(false);
            let path = fs.use_timestamp(true).as_pathbuf(None);
            assert_file_spec(&path, &PathBuf::from("."), true, "log");
        }
        // default() + if_default_use_timestamp(false) + use_timestamp(true) => true
        {
            let mut fs = FileSpec::default();
            fs.if_default_use_timestamp(false);
            let path = fs.use_timestamp(true).as_pathbuf(None);
            assert_file_spec(&path, &PathBuf::from("."), true, "log");
        }
    }

    #[test]
    fn test_from_url() {
        let path = FileSpec::try_from("/a/b/c/d_foo_bar.trc")
            .unwrap()
            .as_pathbuf(None);
        // check folder
        assert_eq!(path.parent().unwrap(), PathBuf::from("/a/b/c"));
        // check filestem
        //  - should start with progname
        let stem = path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .clone()
            .to_string();
        assert_eq!(stem, "d_foo_bar");

        // check suffix
        assert_eq!(path.extension().unwrap(), "trc");
    }

    #[test]
    fn test_basename() {
        {
            let path = FileSpec::try_from("/a/b/c/d_foo_bar.trc")
                .unwrap()
                .o_basename(Some("boo_far"))
                .as_pathbuf(None);
            // check folder
            assert_eq!(path.parent().unwrap(), PathBuf::from("/a/b/c"));

            // check filestem
            //  - should start with progname
            let stem = path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .clone()
                .to_string();
            assert_eq!(stem, "boo_far");

            // check suffix
            assert_eq!(path.extension().unwrap(), "trc");
        }
        {
            let path = FileSpec::try_from("/a/b/c/d_foo_bar.trc")
                .unwrap()
                .o_basename(Option::<String>::None)
                .as_pathbuf(None);
            assert_file_spec(&path, &PathBuf::from("/a/b/c"), false, "trc");
        }
    }

    #[test]
    fn test_directory_and_suffix() {
        {
            let path = FileSpec::try_from("/a/b/c/d_foo_bar.trc")
                .unwrap()
                .directory("/x/y/z")
                .o_suffix(Some("txt"))
                .o_basename(Option::<String>::None)
                .as_pathbuf(None);
            assert_file_spec(&path, &PathBuf::from("/x/y/z"), false, "txt");
        }
    }

    #[test]
    fn test_discriminant() {
        let path = FileSpec::try_from("/a/b/c/d_foo_bar.trc")
            .unwrap()
            .directory("/x/y/z")
            .o_suffix(Some("txt"))
            .o_discriminant(Some("1234"))
            .as_pathbuf(None);
        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            "d_foo_bar_1234.txt"
        );
    }

    #[test]
    fn test_suppress_basename() {
        let path = FileSpec::try_from("/a/b/c/d_foo_bar.trc")
            .unwrap()
            .suppress_basename()
            .o_suffix(Some("txt"))
            .o_discriminant(Some("1234"))
            .as_pathbuf(None);
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "1234.txt");
    }

    #[test]
    fn test_empty_base_name() {
        let path = FileSpec::default()
            .suppress_basename()
            .suppress_timestamp()
            .o_discriminant(Option::<String>::None)
            .as_pathbuf(None);
        assert_eq!(path.file_name().unwrap(), ".log");
    }

    #[test]
    fn test_empty_name() {
        let path = FileSpec::default()
            .suppress_basename()
            .suppress_timestamp()
            .o_suffix(Option::<String>::None)
            .as_pathbuf(None);
        assert!(path.file_name().is_none());
    }

    #[test]
    fn issue_178() {
        let path = FileSpec::default()
            .basename("BASENAME")
            .suppress_timestamp()
            .as_pathbuf(Some(""));
        assert_eq!(path.file_name().unwrap().to_string_lossy(), "BASENAME.log");

        let path = FileSpec::default()
            .basename("BASENAME")
            .discriminant("1")
            .suppress_timestamp()
            .as_pathbuf(Some(""));
        assert_eq!(
            path.file_name().unwrap().to_string_lossy(),
            "BASENAME_1.log"
        );
    }

    #[test]
    fn test_list_of_files() {
        let dir = temp_dir::TempDir::new().unwrap();
        let pd = dir.path();
        let filespec: FileSpec = FileSpec::default()
            .directory(pd)
            .basename("Base")
            .discriminant("Discr")
            .use_timestamp(true);
        println!("Filespec: {}", filespec.as_pathbuf(Some("Infix")).display());

        let mut fn1 = String::new();
        fn1.push_str("Base_Discr_");
        fn1.push_str(&filespec.get_timestamp().unwrap());
        fn1.push_str("_Infix");
        fn1.push_str(".log");
        assert_eq!(
            filespec
                .as_pathbuf(Some("Infix"))
                .file_name()
                .unwrap()
                .to_string_lossy(),
            fn1
        );
        // create typical set of files, and noise
        create_file(pd, "test1.txt");
        create_file(pd, &build_filename(&filespec, "Infix1"));
        create_file(pd, &build_filename(&filespec, "Infix2"));

        println!("\nFolder content:");
        for entry in std::fs::read_dir(pd).unwrap() {
            println!("  {}", entry.unwrap().path().display());
        }

        println!("\nRelevant subset:");
        for pb in filespec.list_of_files(&InfixFilter::StartsWth("Infix".to_string()), Some("log"))
        {
            println!("  {}", pb.display());
        }
    }

    fn build_filename(file_spec: &FileSpec, infix: &str) -> String {
        let mut fn1 = String::new();
        fn1.push_str("Base_Discr_");
        fn1.push_str(&file_spec.get_timestamp().unwrap());
        fn1.push('_');
        fn1.push_str(infix);
        fn1.push_str(".log");
        fn1
    }

    fn create_file(dir: &Path, filename: &str) {
        File::create(dir.join(filename)).unwrap();
    }
}
