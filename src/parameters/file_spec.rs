use crate::{DeferredNow, FlexiLoggerError};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Builder object for specifying the name and path of the log output file.
///
/// The filename is built from several partially optional components, using this pattern:
///
/// ```<basename>[_<discriminant>][_<date>_<time>][_infix][.<suffix>]```
///
/// The default filename pattern without rotation uses the program name as basename,
/// no discriminant, the timestamp of the program start, and the suffix `.log`,
/// e.g.
///
/// ```myprog_2015-07-08_10-44-11.log```.
///
/// This ensures that with every program start a new trace file is written that can easily
/// be associated with a concrete program run.
///
/// When the timestamp is suppressed with [`FileSpec::suppress_timestamp`],
/// you get a fixed output file.
/// It is then worth considering whether a new program start should discard
/// the content of an already existing outputfile or if it should append its new content to it
/// (see [`Logger::append`](crate::Logger::append)).
///
/// With rotation the timestamp is by default suppressed and instead the infix is used.
/// The infix always starts with "_r". For more details how its precise content can be influenced,
/// see [`Naming`](crate::Naming).
///
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileSpec {
    pub(crate) directory: PathBuf,
    pub(crate) basename: String,
    pub(crate) o_discriminant: Option<String>,
    timestamp_cfg: TimestampCfg,
    pub(crate) o_suffix: Option<String>,
    pub(crate) use_utc: bool,
}
impl Default for FileSpec {
    /// Describes a file in the current folder,
    /// using, as its filestem the program name followed by the current timestamp,
    /// and the suffix ".log".
    #[must_use]
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
    ///
    /// # Panics
    ///
    /// Panics if the basename of the given path has no filename
    pub fn try_from<P: Into<PathBuf>>(p: P) -> Result<Self, FlexiLoggerError> {
        let p: PathBuf = p.into();
        if p.is_dir() {
            Err(FlexiLoggerError::OutputBadFile)
        } else {
            Ok(FileSpec {
                directory: p.parent().unwrap(/*cannot fail*/).to_path_buf(),
                basename: p.file_stem().unwrap(/*ok*/).to_string_lossy().to_string(),
                o_discriminant: None,
                o_suffix: p.extension().map(|s| s.to_string_lossy().to_string()),
                timestamp_cfg: TimestampCfg::No,
                use_utc: false,
            })
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

    /// Makes the logger not include a timestamp into the names of the log files
    ///
    /// Equivalent to `use_timestamp(false)`.
    #[must_use]
    pub fn suppress_timestamp(self) -> Self {
        self.use_timestamp(false)
    }

    /// Defines if a timestamp should be included into the names of the log files.
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

    /// Derives a `PathBuf` from the spec and the given infix.
    ///
    /// It is composed like this:
    ///  `<directory>/<basename>_<discr>_<timestamp><infix>.<suffix>`
    #[must_use]
    pub fn as_pathbuf(&self, o_infix: Option<&str>) -> PathBuf {
        let mut filename = self.basename.clone();
        filename.reserve(50);

        if let Some(discriminant) = &self.o_discriminant {
            FileSpec::separate_with_underscore(&mut filename);
            filename.push_str(discriminant);
        }
        if let Some(timestamp) = &self.timestamp_cfg.get_timestamp() {
            FileSpec::separate_with_underscore(&mut filename);
            filename.push_str(timestamp);
        }
        if let Some(infix) = o_infix {
            filename.push_str(infix);
        };
        if let Some(suffix) = &self.o_suffix {
            filename.push('.');
            filename.push_str(suffix);
        }

        let mut p_path = self.directory.clone();
        p_path.push(filename);
        p_path
    }

    fn separate_with_underscore(filename: &mut String) {
        if !filename.is_empty() {
            filename.push('_');
        }
    }

    // <directory>/<basename>_<discr>_<timestamp><infix_pattern>.<suffix>
    pub(crate) fn as_glob_pattern(&self, infix_pattern: &str, o_suffix: Option<&str>) -> String {
        let mut filename = self.basename.clone();
        filename.reserve(50);

        if let Some(discriminant) = &self.o_discriminant {
            FileSpec::separate_with_underscore(&mut filename);
            filename.push_str(discriminant);
        }
        if let Some(timestamp) = &self.timestamp_cfg.get_timestamp() {
            FileSpec::separate_with_underscore(&mut filename);
            filename.push_str(timestamp);
        }
        filename.push_str(infix_pattern);

        match o_suffix {
            Some(s) => {
                filename.push('.');
                filename.push_str(s);
            }
            None => {
                if let Some(suffix) = &self.o_suffix {
                    filename.push('.');
                    filename.push_str(suffix);
                }
            }
        }

        let mut p_path = self.directory.clone();
        p_path.push(filename);
        p_path.to_str().unwrap(/* can hardly fail*/).to_string()
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
    use std::path::{Path, PathBuf};

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
                stem.as_bytes().len(),
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
}
