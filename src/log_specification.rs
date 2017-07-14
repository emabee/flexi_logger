use LogLevelFilter;
use regex::Regex;
use std::env;

/// Immutable struct that defines which loglines are to be written,
/// based on the module, the log level, and the text.
///
/// The loglevel specification via string (relevant for methods parse() and env())
/// works essentially like with env_logger,
/// but we are a bit more tolerant with spaces. Its functionality can be
/// described with some Backus-Naur-form:
///
/// ```text
/// <log_level_spec> ::= single_log_level_spec[{,single_log_level_spec}][/<text_filter>]
/// <single_log_level_spec> ::= <path_to_module>|<log_level>|<path_to_module>=<log_level>
/// <text_filter> ::= <regex>
/// ```
///
/// * If you just specify the module, without log_level, all levels will be traced for this module.
/// * If you just specify a log level, this will be applied as default to all modules without
///   explicit log level assigment.
///   (You see that for modules named error, warn, info, debug or trace,
///   it is necessary to specify their loglevel explicit).
/// * The module names are compared as Strings, with the side effect that a specified module filter
///   affects all modules whose name starts with this String.<br>
///   Example: "foo" affects e.g.
///
///   * foo
///   * foo::bar
///   * foobaz (!)
///   * foobaz::bar (!)
///
/// The optional text filter is applied for all modules.
///
/// Note that external module names are to be specified like in "extern crate ...".
/// For crates with a dash in their name this means: the dash is to be replaced with
/// the underscore (e.g. karl_heinz, not karl-heinz).
pub struct LogSpecification {
    module_filters: Vec<ModuleFilter>,
    textfilter: Option<Regex>,
}

/// Defines which loglevel filter to use for a given module (or as default, if no module is given)
pub struct ModuleFilter {
    pub module_name: Option<String>,
    pub level_filter: LogLevelFilter,
}

impl LogSpecification {
    /// Returns a log specification from a String
    /// (e.g: "crate1, crate2::mod_a, crate3::mod_x = error /foo").
    pub fn parse(spec: &str) -> LogSpecification {
        let mut dirs = Vec::<ModuleFilter>::new();

        let mut parts = spec.split('/');
        let mods = parts.next();
        let filter = parts.next();
        if parts.next().is_some() {
            println!("warning: invalid logging spec '{}', ignoring it (too many '/'s)", spec);
            return LogSpecification::default(LogLevelFilter::Off).finalize();
        }
        mods.map(|m| {
            for s in m.split(',') {
                let s = s.trim();
                if s.len() == 0 {
                    continue;
                }
                let mut parts = s.split('=');
                let (log_level, name) = match (parts.next(),
                                               parts.next().map(|s| s.trim()),
                                               parts.next()) {
                    (Some(part0), None, None) => {
                        if contains_dash(part0) {
                            println!("warning: invalid part in logging spec '{}', contains a \
                                      dash, ignoring it",
                                     part0);
                            continue;
                        }
                        // if the single argument is a log-level string or number,
                        // treat that as a global fallback
                        match part0.trim().parse() {
                            Ok(num) => (num, None),
                            Err(_) => (LogLevelFilter::max(), Some(part0)),
                        }
                    }
                    (Some(part0), Some(""), None) => {
                        if contains_dash(part0) {
                            println!("warning: invalid part in logging spec '{}', contains a \
                                      dash, ignoring it",
                                     part0);
                            continue;
                        }

                        (LogLevelFilter::max(), Some(part0))
                    }
                    (Some(part0), Some(part1), None) => {
                        if contains_dash(part0) {
                            println!("warning: invalid part in logging spec '{}', contains a \
                                      dash, ignoring it",
                                     part0);
                            continue;
                        }
                        match part1.trim().parse() {
                            Ok(num) => (num, Some(part0.trim())),
                            _ => {
                                println!("warning: invalid part in logging spec '{}', ignoring it",
                                         part1);
                                continue;
                            }
                        }
                    }
                    _ => {
                        println!("warning: invalid part in logging spec '{}', ignoring it", s);
                        continue;
                    }
                };
                dirs.push(ModuleFilter {
                    module_name: name.map(|s| s.to_string()),
                    level_filter: log_level,
                });
            }
        });

        let textfilter = filter.map_or(None, |filter| match Regex::new(filter) {
            Ok(re) => Some(re),
            Err(e) => {
                println!("warning: invalid regex filter - {}", e);
                None
            }
        });

        LogSpecification {
            module_filters: dirs.level_sort(),
            textfilter: textfilter,
        }
    }

    /// Returns a log specification based on the value of the environment variable RUST_LOG,
    /// or an empty one.
    pub fn env() -> LogSpecification {
        match env::var("RUST_LOG") {
            Ok(spec) => LogSpecification::parse(&spec),
            Err(..) => LogSpecification::default(LogLevelFilter::Off).finalize(),
        }
    }

    /// Creates a LogSpecBuilder, setting the default log level.
    pub fn default(llf: LogLevelFilter) -> LogSpecBuilder {
        LogSpecBuilder {
            module_filters: vec![ModuleFilter {
                                     module_name: None,
                                     level_filter: llf,
                                 }],
        }
    }

    /// Provides a reference to the module filters.
    pub fn module_filters(&self) -> &Vec<ModuleFilter> {
        &self.module_filters
    }

    /// Provides a reference to the text filter.
    pub fn text_filter(&self) -> &Option<Regex> {
        &self.textfilter
    }
}

fn contains_dash(s: &str) -> bool {
    s.find('-') != None
}

/// Builder for LogSpecification.
pub struct LogSpecBuilder {
    module_filters: Vec<ModuleFilter>,
}

impl LogSpecBuilder {
    /// Adds a log level filter for a module.
    pub fn module(&mut self, module_name: &str, lf: LogLevelFilter) -> &mut LogSpecBuilder {
        self.module_filters.push(ModuleFilter {
            module_name: Some(module_name.to_string()),
            level_filter: lf,
        });
        self
    }

    /// Creates a log specification without text filter.
    pub fn finalize(self) -> LogSpecification {
        LogSpecification {
            module_filters: self.module_filters.level_sort(),
            textfilter: None,
        }
    }

    /// Creates a log specification with text filter.
    pub fn finalize_with_textfilter(self, tf: Regex) -> LogSpecification {
        LogSpecification {
            module_filters: self.module_filters.level_sort(),
            textfilter: Some(tf),
        }
    }
}

trait LevelSort {
    fn level_sort(self) -> Vec<ModuleFilter>;
}
impl LevelSort for Vec<ModuleFilter> {
    /// Sort the module filters by length of their name,
    /// this allows a little more efficient lookup at runtime.
    fn level_sort(mut self) -> Vec<ModuleFilter> {
        self.sort_by(|a, b| {
            // let alen = a.module_name.as_ref().map(|a| a.len()).unwrap_or(0);
            // let blen = b.module_name.as_ref().map(|b| b.len()).unwrap_or(0);
            // alen.cmp(&blen)
            a.module_name.cmp(&b.module_name)
        });
        self
    }
}


#[cfg(test)]
mod tests {
    extern crate log;
    use LogSpecification;
    use log::LogLevelFilter;

    #[test]
    fn parse_logging_spec_valid() {
        let spec = LogSpecification::parse("crate1::mod1=error,crate1::mod2,crate2=debug");
        assert_eq!(spec.module_filters().len(), 3);
        assert_eq!(spec.module_filters()[0].module_name, Some("crate1::mod1".to_string()));
        assert_eq!(spec.module_filters()[0].level_filter, LogLevelFilter::Error);

        assert_eq!(spec.module_filters()[1].module_name, Some("crate1::mod2".to_string()));
        assert_eq!(spec.module_filters()[1].level_filter, LogLevelFilter::max());

        assert_eq!(spec.module_filters()[2].module_name, Some("crate2".to_string()));
        assert_eq!(spec.module_filters()[2].level_filter, LogLevelFilter::Debug);

        assert!(spec.text_filter().is_none());
    }

    #[test]
    fn parse_logging_spec_invalid_crate() {
        // test parse_logging_spec with multiple = in specification
        let spec = LogSpecification::parse("crate1::mod1=warn=info,crate2=debug");
        assert_eq!(spec.module_filters().len(), 1);
        assert_eq!(spec.module_filters()[0].module_name, Some("crate2".to_string()));
        assert_eq!(spec.module_filters()[0].level_filter, LogLevelFilter::Debug);
        assert!(spec.text_filter().is_none());
    }

    #[test]
    fn parse_logging_spec_invalid_log_level() {
        // test parse_logging_spec with 'noNumber' as log level
        let spec = LogSpecification::parse("crate1::mod1=noNumber,crate2=debug");
        assert_eq!(spec.module_filters().len(), 1);
        assert_eq!(spec.module_filters()[0].module_name, Some("crate2".to_string()));
        assert_eq!(spec.module_filters()[0].level_filter, LogLevelFilter::Debug);
        assert!(spec.text_filter().is_none());
    }

    #[test]
    fn parse_logging_spec_string_log_level() {
        // test parse_logging_spec with 'warn' as log level
        let spec = LogSpecification::parse("crate1::mod1=wrong, crate2=warn");
        assert_eq!(spec.module_filters().len(), 1);
        assert_eq!(spec.module_filters()[0].module_name, Some("crate2".to_string()));
        assert_eq!(spec.module_filters()[0].level_filter, LogLevelFilter::Warn);
        assert!(spec.text_filter().is_none());
    }

    #[test]
    fn parse_logging_spec_empty_log_level() {
        // test parse_logging_spec with '' as log level
        let spec = LogSpecification::parse("crate1::mod1=wrong, crate2=");
        assert_eq!(spec.module_filters().len(), 1);
        assert_eq!(spec.module_filters()[0].module_name, Some("crate2".to_string()));
        assert_eq!(spec.module_filters()[0].level_filter, LogLevelFilter::max());
        assert!(spec.text_filter().is_none());
    }

    #[test]
    fn parse_logging_spec_global() {
        // test parse_logging_spec with no crate
        let spec = LogSpecification::parse("warn,crate2=debug");
        assert_eq!(spec.module_filters().len(), 2);
        assert_eq!(spec.module_filters()[0].module_name, None);
        assert_eq!(spec.module_filters()[0].level_filter, LogLevelFilter::Warn);
        assert_eq!(spec.module_filters()[1].module_name, Some("crate2".to_string()));
        assert_eq!(spec.module_filters()[1].level_filter, LogLevelFilter::Debug);
        assert!(spec.text_filter().is_none());
    }

    #[test]
    fn parse_logging_spec_valid_filter() {
        let spec = LogSpecification::parse(" crate1::mod1 = error , crate1::mod2,crate2=debug/abc");
        assert_eq!(spec.module_filters().len(), 3);
        assert_eq!(spec.module_filters()[0].module_name, Some("crate1::mod1".to_string()));
        assert_eq!(spec.module_filters()[0].level_filter, LogLevelFilter::Error);

        assert_eq!(spec.module_filters()[1].module_name, Some("crate1::mod2".to_string()));
        assert_eq!(spec.module_filters()[1].level_filter, LogLevelFilter::max());

        assert_eq!(spec.module_filters()[2].module_name, Some("crate2".to_string()));
        assert_eq!(spec.module_filters()[2].level_filter, LogLevelFilter::Debug);
        assert!(spec.text_filter().is_some() &&
                spec.text_filter().as_ref().unwrap().to_string() == "abc");
    }

    #[test]
    fn parse_logging_spec_invalid_crate_filter() {
        let spec = LogSpecification::parse("crate1::mod1=error=warn,crate2=debug/a.c");
        assert_eq!(spec.module_filters().len(), 1);
        assert_eq!(spec.module_filters()[0].module_name, Some("crate2".to_string()));
        assert_eq!(spec.module_filters()[0].level_filter, LogLevelFilter::Debug);
        assert!(spec.text_filter().is_some() &&
                spec.text_filter().as_ref().unwrap().to_string() == "a.c");
    }

    #[test]
    fn parse_logging_spec_invalid_crate_with_dash() {
        let spec = LogSpecification::parse("karl-heinz::mod1=warn,crate2=debug/a.c");
        assert_eq!(spec.module_filters().len(), 1);
        assert_eq!(spec.module_filters()[0].module_name, Some("crate2".to_string()));
        assert_eq!(spec.module_filters()[0].level_filter, LogLevelFilter::Debug);
        assert!(spec.text_filter().is_some() &&
                spec.text_filter().as_ref().unwrap().to_string() == "a.c");
    }

    #[test]
    fn parse_logging_spec_empty_with_filter() {
        let spec = LogSpecification::parse("crate1/a*c");
        assert_eq!(spec.module_filters().len(), 1);
        assert_eq!(spec.module_filters()[0].module_name, Some("crate1".to_string()));
        assert_eq!(spec.module_filters()[0].level_filter, LogLevelFilter::max());
        assert!(spec.text_filter().is_some() &&
                spec.text_filter().as_ref().unwrap().to_string() == "a*c");
    }
}
