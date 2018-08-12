# Change log for flexi_logger
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/) and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.1] - 2018-08-12
Introduce `Logger::duplicate_to_stderr()`, as a more flexible replacement for `duplicate_error()` and `duplicate_info()`. 

## [0.9.0] - 2018-07-06 
### Eliminate String allocation
Get rid of the unneccessary String allocation we've been 
carrying with us since ages. This implies changing the signature of the format functions.

In case you provide your own format function, you'll need to adapt it to the new signature. 
Luckily, the effort is low.

As an example, here is how the definition of the `opt_format` function changed:
```rust
- pub fn opt_format(record: &Record) -> String {
-     format!(
---
+ pub fn opt_format(w: &mut io::Write, record: &Record) -> Result<(), io::Error> {
+     write!(
+         w,
```

Similarly, if you're using the advanced feature of providing your own implementation of LogWriter, 
you need to adapt it. The change again is trivial, and should even slightly 
simplify your code (you can return io errors and don't have to catch them yourself).

### Misc
The docu generation on docs.rs is now configured to considers all features, we thus 
expose `Logger.start_with_specfile()` only if the specfile feature is used. So we can revert the change done with 0.8.1.

## [0.8.4] - 2018-06-18
Add flexi_logger to category `development-tools::debugging`

## [0.8.3] - 2018-05-14
Make append() also work for rotating log files

## [0.8.2] - 2018-04-03
Add option to append to existing log files, rather than always truncating them

## [0.8.1] - 2018-3-19
Expose `Logger.start_with_specfile()` always
...and not only if the feature "specfile" is used - otherwise it does not appear 
in the auto-generated docu (because it does not use --allfeatures)

## [0.8.0] - 2018-03-18 
Add specfile feature
* Add a feature that allows to specify the LogSpecification via a file 
  that can be edited while the program is running
* Remove/hide deprecated APIs
* As a consequence, cleanup code, get rid of duplicate stuff.

## [0.7.1] - 2018-03-07
Bugfix: do not create empty files when used in env_logger style.
Update docu and the description in cargo.toml

## [0.7.0] - 2018-02-25
Add support for multiple log output streams
- replace FlexiWriter with DefaultLogWriter, which wraps a FileLogWriter
- add test where a SecurityWriter and an AlertWriter are added
- add docu
- move deprecated structs to separate package
- move benches to folder benches

## [0.6.13] 2018-02-09
Add Logger::with_env_or_str()

## [0.6.12] 2018-2-07
Add ReconfigurationHandle::parse_new_spec()

## [0.6.11] 2017-12-29
Fix README.md

## [0.6.10] 2017-12-29
Publish version based on log 0.4

## ...

## [0.6.0] 2017-07-13
Use builder pattern for LogSpecification and Logger
- deprecate outdated API
- "objectify" LogSpecification
- improve documentation, e.g. document the dash/underscore issue
