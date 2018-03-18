# Change log for flexi_logger

## [0.8.0] Add specfile feature
* Add a feature that allows to specify the LogSpecification via a file 
  that can be edited while the program is running
* Remove/hide deprecated APIs
* As a consequence, cleanup code, get rid of duplicate stuff.

## [0.7.1] (bugfix)  do not create empty files when used in env_logger style
Update docu and the description in cargo.toml

## [0.7.0] Add support for multiple log output streams
- replace FlexiWriter with DefaultLogWriter, which wraps a FileLogWriter
- add test where a SecurityWriter and an AlertWriter are added
- add docu
- move deprecated structs to separate package
- move benches to folder benches

## [0.6.13] Add Logger::with_env_or_str()

## [0.6.12] Add ReconfigurationHandle::parse_new_spec()

## [0.6.10] Publish version based on log 0.4


## [0.6.0] Use builder pattern for LogSpecification and Logger
- deprecate outdated API
- "objectify" LogSpecification
- improve documentation, e.g. document the dash/underscore issue
