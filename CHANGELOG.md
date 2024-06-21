# Changelog for flexi_logger

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/) and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.28.5] - 2024-06-21

Remove unnecessary dependency to `is-terminal`.

Add impl `From<LevelFilter>` for `LogSpecification`.

Kudos to [Oakchris1955](https://github.com/Oakchris1955).

## [0.28.4] - 2024-06-14

Fix [issue #162](https://github.com/emabee/flexi_logger/issues/162)
(FileLogWriter does not follow its max_level), kudos to [JoeWildfong](https://github.com/JoeWildfong).

## [0.28.3] - 2024-06-10

Add special handling for empty current infix to `Naming::TimestampsCustomFormat`
([issue #161](https://github.com/emabee/flexi_logger/issues/161)).

## [0.28.2] - 2024-06-09

Add variant `Naming::TimestampsCustomFormat` ([issue #158](https://github.com/emabee/flexi_logger/issues/158)),
kudos to [jb-alvarado](https://github.com/jb-alvarado).

## [0.28.1] - 2024-06-01

Introduce `flexi_logger::init()` as super-minimal entry usage.

Update dependencies.

## [0.28.0] - 2024-03-16

Detach from `lazy_static`, use `std::sync::OnceLock` instead.

Bump minimal supported rust version to 1.70.

If `flexi_logger` runs into issues itself, it will try to write error messages into the configured
error output channel. By default, `flexi_logger` panics if writing to the error output channel fails.
It is now possible to gracefully "swallow" the error messages and continue
(see [panic_if_error_channel_is_broken](https://docs.rs/flexi_logger/latest/flexi_logger/struct.Logger.html#method.panic_if_error_channel_is_broken)).

The new feature `kv` allows making use of the `kv` feature of `log` together with `flexi_logger`s
format functions, and adds a dependency to `log/kv_serde`.

The new feature `json` adds a format function `json_format` and dependencies to `serde_json`,
`serde` and `serde_derive`.

## [0.27.4] - 2024-01-20

Add ability to omit the basename cleanly, without leading underscore
([issue #153](https://github.com/emabee/flexi_logger/issues/153),
kudos to [krystejj](https://github.com/krystejj).

## [0.27.3] - 2023-11-10

Fix [issue #152](https://github.com/emabee/flexi_logger/issues/152).

## [0.27.2] - 2023-09-27

Fix wrong timestamp handling for the second rotation (second part of
[issue #150](https://github.com/emabee/flexi_logger/issues/150)).

## [0.27.1] - 2023-09-27

Fix issues with sub-second rotations and with cleanup when all logfiles should be compressed
([issue #150](https://github.com/emabee/flexi_logger/issues/150)).

## [0.27.0] - 2023-09-20

Revise, and modify the signature of, `LoggerHande::existing_log_files()` (version bump).

Extend the trait `LogWriter` with an optional method `rotate`.

Extend impact of `LoggerHande::trigger_rotation()` to all configured writers.

## [0.26.1] - 2023-09-19

Introduce new naming variants that work without `_rCURRENT` files: `Naming::TimestampsDirect`
and `Naming::NumbersDirect` (delivers #127).

Improve documentation of filename handling.

Introduce `LoggerHandle.trigger_rotation()` (delivers #147).

## [0.26.0] - 2023-08-30

Re-open output also for other writers (delivers #143).

Rename method to re-open output from LoggerHandle (leads to version bump).

Use `dep:` in Cargo.toml for references to most dependencies, in order to avoid implicit "features".

Fix #145 (minor internal optimization).

## [0.25.6] - 2023-07-28

Add methods
`LoggerHandle::adapt_duplication_to_stderr` and  `LoggerHandle::adapt_duplication_to_stdout`
(realizes issue #142).

Extend docu on providing custom format.

Use rust-script instead of cargo-script for qualification scripts.

Update dependencies.

## [0.25.5] - 2023-05-25

Use display (rather than debug) formatting for thread names
(kudos to [mpalmer](https://github.com/mpalmer)).

## [0.25.4] - 2023-05-05

Add `LoggerHandle::existing_log_files()`.

## [0.25.3] - 2023-03-04

Introduce additional `WriteMode` variant `SupportCapture`.

## [0.25.2] - 2023-03-02

Replace dependency `atty` with `is-terminal`, due to
[RUSTSEC-2021-0145](https://rustsec.org/advisories/RUSTSEC-2021-0145).

## [0.25.1] - 2023-02-06

Use chrono's support for rfc3339. Improve tests for `DeferredNow`.

## [0.25.0] - 2023-02-03

Fix issues #132 and #133.

Update dependencies.

Bump MSRV to 1.60, because toml needs it now.

Improve documentation of feature dependencies.

Minor stuff.

## [0.24.2] - 2022-12-15

Move from unmaintained `ansi_term` to `nu-ansi-term`.

Fix new clippies.

## [0.24.1] - 2022-11-01

Some improvements in respect to `use_utc`:

- add method DeferredNow::now_utc_owned()
- documentation
- test improvement

## [0.24.0] - 2022-10-06

Revert back to using `chrono`, since `chrono` is now fortunately maintained again and its timezone
handling is fixed meanwhile

- this change largely reverts the changes done for [0.19.6]
- a version bump is necessary since this affects the API, e.g. in `DeferredNow`
- the feature `use_chrono_for_offset` became obsolete and is removed

On linux and Mac, improve the logic that handles the issue described again in
[issue-122](https://github.com/emabee/flexi_logger/issues/122).

## [0.23.3] - 2022-09-11

Re-introduce `LoggerHandle::clone()`.

## [0.23.2] - 2022-09-06

Fix security advisory (see #117) by replacing the dependency from `notify 4.0` with
`notify-debouncer-mini 0.2` (which depends on `notify 5.0`). As a side-effect,
the thread `flexi_logger-specfile-watcher` is replaced with `notify-rs debouncer loop`.

Adapt and simplify the submodule `trc` a bit.

## [0.23.1] - 2022-09-02

Fix a panic that can happen if `Naming::Timestamps` and `FileSpec::o_suffix(None)` are used and
rotation happens within a second ([issue-116](https://github.com/emabee/flexi_logger/issues/116)).

Bump MSRV to 1.59 (because the `time` crate did this).

## [0.23.0] - 2022-08-04

Switch to edition 2021, use latest patch of `time` version "0.3",
bump minimal supported rust version to "1.57.0".

## [0.22.6] - 2022-08-03

Add interconversions between log::LevelFilter and flexi_logger::Duplicate
(kudos to [rlee287](https://github.com/rlee287)).

## [0.22.5] - 2022-06-03

Only depend on the parts of crossbeam that are used (kudos to
[bsilver8192](https://github.com/bsilver8192)).

## [0.22.4] - 2022-06-03

Add support for Rfc3164 to `SyslogWriter` (kudos to [mbodmer](https://github.com/mbodmer)).

Add `Clone` and `Copy` implementations to enum Duplicate (kudos to
[ComplexSpaces](complexspacescode@gmail.com)).

## [0.22.3] -  2022-02-01

Code maintenance: remove the feature "external_rotation".

Bump minimal version of `time` crate to "0.3.7".

## [0.22.2] - 2022-01-08

Add `LoggerHandle::reopen_outputfile` and deprecate feature `external_rotation`.

## [0.22.1] - 2022-01-05

Enable symlink on all unix platforms, not just linux.

Rework the optional syslog writer (kudos to [ObsceneGiraffe](https://github.com/ObsceneGiraffe)):

- bugfix: write only full lines
- use owned buffer to avoid allocations
- encapsulate implementation details
- remove additional buffer from `SyslogConnector::Tcp`

Add method `LoggerHandle::flw_config` (kudos to [Ivan Azoyan](https://github.com/azoyan)).

Reduce the used feature-list of the optional dependency chrono
(to get rid of an indirect dependency to an old time version).

Add feature `external_rotation`.

## [0.22.0] - 2021-12-12

Improve the option to use UTC for all timestamps (in filenames and log lines)
(<https://docs.rs/flexi_logger/latest/flexi_logger/struct.Logger.html#method.use_utc>) such that
the error message regarding a failed offset detection is not provoked if UTC is enforced.

The API modification done in 0.21.0 to `DeferredNow` is reverted.

## [0.21.0] - 2021-12-10

Add option to use UTC for all timestamps (in filenames and log lines).

## [0.20.1] - 2021-11-18

Add the optional feature `use_chrono_for_offset` as a workaround for the current behavior
of `time` on unix.

Add an option to configure the error output channel.

## [0.20.0] - 2021-11-13

Switch to `time 0.3.5`, and retrieve the UTC offset while `flexi_logger` is initialized.
See also `time`'s [CHANGELOG](https://github.com/time-rs/time/blob/main/CHANGELOG.md#035-2021-11-12).

**Reason for the version bump**:

The inner representation of `DeferredNow` has changed from `chrono::DateTime<Local>`
to `time::OffsetDateTime`, and this is visible e.g. to implementors of format functions.

## [0.19.6] - 2021-10-26

Use `time` directly, instead of `chrono`,
due to [RUSTSEC-2020-0159](https://rustsec.org/advisories/RUSTSEC-2020-0159).
Bumps the minimal supported rust version to 1.51.0. Improves performance a bit.

Unfortunately, this version suffers on linux from `time`'s somewhat radical behavior
to not support UTC offsets on linux.

## [0.19.5] - 2021-10-19

Remove time 0.1 from dependency tree
(see [PR 96](https://github.com/emabee/flexi_logger/issues/96)) -
kudos to [complexspaces](https://github.com/complexspaces)!

Add feature `dont_minimize_extra_stacks`
(fixes [issue-95](https://github.com/emabee/flexi_logger/issues/95)) -
kudos to [leishiao](https://github.com/leishiao)!

## [0.19.4] - 2021-09-15

Fix [issue-94](https://github.com/emabee/flexi_logger/issues/94) -
kudos to [leishiao](https://github.com/leishiao)!

## [0.19.0] - [0.19.3] - 2021-09-10

Platform-specific fixes, and introduction of github-actions-based CI.
Kudos to [dallenng](https://github.com/dallenng) and [HEnquist](https://github.com/HEnquist)!

`FileLogWriter` has been functionally extended to make it usable "stand-alone".
As part of that, the `FlWriteMode` is gone, and the normal `WriteMode` is used.

`WriteMode::BufferDontFlushWith` was added.

A new experimental feature (and module) "trc" allows using `flexi_logger` functionality
with `tracing`.

Error handling is improved, error codes are documented comprehensively,
errors now also print a link to the error documentation.

Default color for DEBUG lines was changed
(fixes [issue-88](https://github.com/emabee/flexi_logger/issues/88), kudos goes to [HEnquist](https://github.com/HEnquist)!).

Test coverage is improved.

## [0.18.1] - 2021-08-27

Implement async mode also for `log_to_stdout()` and `log_to_stderr()`.

## [0.18.0] - 2021-06-02

Significant API revision, to better cope with new features and for better readability/applicability.

Most important changes:

- Better error handling in factory methods:
  - `Logger::with_env()` is replaced with `Logger::try_with_env()`, which returns a `Result`
  - `Logger::with_str()` is replaced with `Logger::try_with_str()`, which returns a `Result`
  - `Logger::with_env_or_str()` is replaced with `Logger::try_with_env_or_str()`,
    which returns a `Result`
  - consequently, the method `Logger::check_parser_error` is gone
- Bundling file-related aspects
  - introduction of `FileSpec`
  - move of filename-related methods from `Logger` to `FileSpec`
    (and similarly on the `FileLogWriter`)
- `Logger::log_target(LogTarget)` is replaced with a set of methods
  - `Logger::log_to_file(FileSpec)`
  - `Logger::log_to_stdout()`
  - `Logger::log_to_stderr()`
  - `Logger::log_to_writer(Box<dyn LogWriter>)`
  - `Logger::log_to_file_and_writer(FileSpec,Box<dyn LogWriter>)`
  - `Logger::do_not_log()`
- The new method
  [`Logger::write_mode(WriteMode)`](https://docs.rs/flexi_logger/latest/flexi_logger/struct.Logger.html#method.buffer_and_flush)
  - replaces several methods to control buffer handling etc
  - offers additionally **asynchronous file I/O** (if the crate feature `async` is used)
- Keeping the `LoggerHandle` alive has become crucial (except for trivial cases)!
- Several methods are now more generic with their input parameters
- A new method `LoggerHandle::reset_flw` allows reconfiguring a used `FileLogWriter` at runtime

Added an option to apply a stateful filter before log lines are really written
(kudos to jesdazrez (Jesús Trinidad Díaz Ramírez)!).

Fixed error handling in logspec parsing (wrong error was thrown).

Several docu improvements.

## [0.17.1] - 2021-01-14

Add options `Logger::buffer_and_flush()` and `buffer_and_flush_with()`
as means to avoid long output delays.

## [0.17.0] - 2021-01-08

Introduce optional buffering of log output. This increases performance
(which can be relevant for programs with really high log production),
but delays log line appearance in the output, which can be confusing,
and requires to flush or shutdown the logger at the end of the program
to ensure that all logs are written to the output before
the program terminates.

Reduce the size of `LogConfiguration` considerably by moving the optional and rarely used textfilter
into the heap. This unfortunately leads to an incompatible change in a rarely used public method
(`LogConfiguration::text_filter()` was returning a `&Option<Regex>`,
and is now returning `Option<&Regex>`), which enforces a version bump.

Rename ReconfigurationHandle to LoggerHandle (and add an type alias with the old name).

Add the public method `LoggerHandle::flush()`.

Expose `DeferredNow::new()`.

Add some `must_use` annotations where appropriate.

## [0.16.2] - 2020-11-18

Add module
[code-examples](https://docs.rs/flexi_logger/latest/flexi_logger/code_examples/index.html)
with additional usage documentation.
This is a follow-up of a PR, kudos goes to [devzbysiu](https://github.com/devzbysiu)!

## [0.16.1] - 2020-09-20

Support empty toml spec files (kudos to ijackson for
[pull request 66](https://github.com/emabee/flexi_logger/pull/66)!)
(was supposed to be part of 0.16.0, but I had forgotten to merge it).

## [0.16.0] - 2020-09-19

If file logging is used, do not create the output file if no log is written.
Solves [issue-62](https://github.com/emabee/flexi_logger/issues/62).

Improve color handling

- introduce AdaptiveFormat for a clearer API
- Support using feature `atty` without provided coloring
- Extend example `colors` to provide insight in how AdaptiveFormat works
- Remove the deprecated method `Logger::do_not_log()`;
  use `log_target()` with `LogTarget::DevNull` instead.
- Remove deprecated method `Logger::o_log_to_file()`; use  `log_target()` instead.
  The clearer convenience method `Logger::log_to_file()` is still available.

Improve the compression feature. Solves [issue-65](https://github.com/emabee/flexi_logger/issues/65).

- breaking change: change the file suffix for the compressed log files from `.zip` to `.gz`
- Fix wrong wording in code and documentation
- deprecate the feature name `ziplog` and call the feature now `compress`
- rename `Cleanup::KeepZipFiles` into `Cleanup::KeepCompressedFiles`
   and `Cleanup::KeepLogAndZipFiles` into `Cleanup::KeepLogAndCompressedFiles`
  - the old names still work but are deprecated

If file logging is used, do not create the output file if no log is written
Solves issue [issue-62](https://github.com/emabee/flexi_logger/issues/62).

## [0.15.12] - 2020-28-08

Make `1.37.0` the minimal rust version for `flexi_logger`.

## [0.15.11] - 2020-08-07

Introduce feature `specfile_without_notification` to allow coping with OS issues
(solves [issue-59](https://github.com/emabee/flexi_logger/issues/59)).

## [0.15.10] - 2020-07-22

Minor code maintenance.

## [0.15.9] - 2020-07-21

Allow using the log target with fantasy names, like with `env_logger`.
Solves [issue-56](https://github.com/emabee/flexi_logger/issues/56).

## [0.15.8] - 2020-07-20

Allow modifying the coloring palette through the environment variable `FLEXI_LOGGER_PALETTE`.
See function [style](https://docs.rs/flexi_logger/latest/flexi_logger/fn.style.html) for details.
Solves [issue-55](https://github.com/emabee/flexi_logger/issues/55).

By default, don't use colors if stdout or stderr are not a terminal
Solves [issue-57](https://github.com/emabee/flexi_logger/issues/57).

Add variant Criterion::AgeOrSize
(kudos to
[pscott](https://github.com/pscott)!,
[PR-54](https://github.com/emabee/flexi_logger/pull/54)).

## [0.15.7] - 2020-07-02

Add some Debug derives
(kudos to
[pscott](https://github.com/pscott)!,
[PR-52](https://github.com/emabee/flexi_logger/pull/52)).

## [0.15.6] - 2020-07-02

Introduce separate formatting for stdout
(kudos to
[pscott](https://github.com/pscott)!,
[PR-51](https://github.com/emabee/flexi_logger/pull/51)).

Deprecate `Logger::do_not_log()`.

## [0.15.5] - 2020-06-18

Add `Logger::duplicate_to_stdout()` to fix
[issue-47](https://github.com/emabee/flexi_logger/issues/47).

## [0.15.4] - 2020-06-09

Fix [issue-45](https://github.com/emabee/flexi_logger/issues/45), which was a panic in
the specfile watcher when some log files were deleted manually while the program was running
(kudos to
[avl](https://github.com/avl)!,
[PR-46](https://github.com/emabee/flexi_logger/pull/46)).

## [0.15.3] - 2020-06-04

Add compatibility with multi_log by adding methods
`Logger::build` and `Logger::build_with_specfile` (fixes issue-44).

Add `LogSpecBuilder::insert_modules_from()` (fixes issue-43).

## [0.15.2] - 2020-03-24

Improve handling of parse-errors.

Fix default format for files (was and is documented to be uncolored, but was colored).

## [0.15.1] - 2020-03-04

Make the textfilter functionality an optional default feature;
deselecting it removes the regex crate as a required dependency,
which reduces the size overhead for any binary using `flexi_logger`
(kudos to [Petre Eftime](petre.eftime@gmail.com)!).

## [0.15.0] - 2020-02-27

Refine and rename error variants to allow e.g. differentiating
between errors related to the output (files)
and errors related to the specfile.

## [0.14.8] - 2020-02-06

Make cleanup more robust, and allow controlling the cleanup-thread also with
`Logger::start_with_specfile()`.

## [0.14.7] - 2020-02-04

If rotation is used with cleanup, do the cleanup by default in a background thread
(solves [issue 39](https://github.com/emabee/flexi_logger/issues/39)).

For the ziplog feature, switch from `zip` crate to `flate2`.

## [0.14.6] - 2020-01-28

Fix [issue 38](https://github.com/emabee/flexi_logger/issues/38)
(Old log files are not removed if rCURRENT doesn't overflow).

## [0.14.5] - 2019-11-06

Pass format option into custom loggers (pull request 37).

## [0.14.4] - 2019-09-25

Fix bug in specfile handling ([issue 36](https://github.com/emabee/flexi_logger/issues/36)).

Improve docu and implementation of create_symlink.

Minor other stuff.

## [0.14.3] - 2019-08-04

Allow defining custom handlers for the default log target
(solves [issue 32](https://github.com/emabee/flexi_logger/issues/32)).

## [0.14.2] - 2019-08-04

Use implicit locking of stderr in StdErrWriter.

Allow failures in travis' windows build.

Add license files.

## [0.14.1] - 2019-08-04

Support recursive logging also with FileLogWriter, sharing the buffer with the PrimaryWriter.

Fix multi-threading issue (incorrect line-break handling with stderr).

## [0.14.0] - 2019-07-22

Further stabilize the specfile feature.

Remove `LogSpecification::ensure_specfile_exists()` and `LogSpecification::from_file()`
from public API, where they should not be (-> version bump).

Harmonize all eprintln! calls to
prefix the output with "`[flexi_logger]` ".

## [0.13.4] - 2019-07-19

Only relevant for the `specfile` feature:
initialize the logger before dealing in any way with the specfile,
and do the initial read of the specfile in the main thread,
i.e. synchronously, to ensure a deterministic behavior during startup
(fixes [issue 31](https://github.com/emabee/flexi_logger/issues/31)).

## [0.13.3] - 2019-07-08

Improve the file watch for the specfile to make the `specfile` feature more robust.
E.g. allow editing the specfile on linux
with editors that move the original file to a backup name.

Add an option to write the log to stdout, as recommended for
[twelve-factor apps](https://12factor.net/logs).

## [0.13.2] - 2019-06-02

Make get_creation_date() more robust on all platforms.

## [0.13.1] - 2019-05-29

Fix fatal issue with get_creation_date() on linux
(see <https://github.com/emabee/flexi_logger/pull/30>).

## [0.13.0] - 2019-05-28

Improve performance for plain stderr logging.

Improve robustnesss for recursive log calls.

## [0.12.0] - 2019-05-24

Revise handling of record.metadata().target() versus record.module_path().

Incompatible API modification: Logger.rotate() takes now three parameters.

Suppport different formatting for stderr and files.

Add feature `colors` (see `README.md` for details).

Remove the deprecated `Logger::start_reconfigurable()` and `Logger::rotate_over_size()`.

## [0.11.5] - 2019-05-15

Fix [issue 26](https://github.com/emabee/flexi_logger/issues/26) (logging off for specific modules).

Fix [issue 27](https://github.com/emabee/flexi_logger/issues/27) (log files blank after restart).

Fix [issue 28](https://github.com/emabee/flexi_logger/issues/28)
(add a corresponding set of unit tests to FileLogWriter).

## [0.11.4] - 2019-04-01

Version updates of dependencies.

## [0.11.3] - 2019-03-28

Add SyslogWriter.

## [0.11.2] - 2019-03-22

Change API to more idiomatic parameter types, in a compatible way.

Add first implementation of a SyslogWriter.

## [0.11.1] - 2019-03-06

Add option to write windows line endings, rather than a plain `\n`.

## [0.11.0] - 2019-03-02

Add options to cleanup rotated log files, by deleting and/or zipping older files.

Remove some deprecated methods.

## [0.10.7] - 2019-02-27

Let the BlackHoleLogger, although it doesn't write a log, still duplicate to stderr.

## [0.10.6] - 2019-02-26

Deprecate `Logger::start_reconfigurable()`, let `Logger::start()` return a reconfiguration handle.

Add an option to write all logs to nowhere (i.e., do not write any logs).

## [0.10.5] - 2019-01-15

Eliminate performance penalty for using reconfigurability.

## [0.10.4] - 2019-01-07

Add methods to modify the log spec temporarily.

## [0.10.3] - 2018-12-08

Advance to edition 2018.

## [0.10.2] - 2018-12-07

Log-spec parsing is improved, more whitespace is tolerated.

## [0.10.1] - 2018-11-08

When file rotation is used, the name of the file to which the logs are written is now stable.

Details:

- the logs are always written to a file with infix _rCURRENT
- if this file exceeds the specified rotate-over-size, it is closed and renamed
  to a file with a sequential number infix, and then the logging continues again
  to the (fresh) file with infix _rCURRENT

Example:

After some logging with your program my_prog, you will find files like

```text
my_prog_r00000.log
my_prog_r00001.log
my_prog_r00002.log
my_prog_rCURRENT.log
```

## [0.10.0] - 2018-10-30

`LogSpecification::parse()` now returns a `Result<LogSpecification, FlexiLoggerError>`, rather than
a log spec directly (-> version bump).
This enables a more reliable usage of FlexiLogger in non-trivial cases.

For the sake of compatibility for the normal usecases, the Logger methods `with_str()` etc.
remain unchanged. An extra method is added to retrieve parser errors, if desired.

## [0.9.3] - 2018-10-27

Docu improvement.

## [0.9.2] - 2018-08-13

Fix incorrect filename generation with rotation,
i.e., switch off timestamp usage when rotation is used.

## [0.9.1] - 2018-08-12

Introduce `Logger::duplicate_to_stderr()`, as a more flexible replacement for `duplicate_error()`
and `duplicate_info()`.

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
expose `Logger.start_with_specfile()` only if the specfile feature is used. So we can revert the
change done with 0.8.1.

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

- Add a feature that allows to specify the LogSpecification via a file
  that can be edited while the program is running
_ Remove/hide deprecated APIs
- As a consequence, cleanup code, get rid of duplicate stuff.

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

Add Logger::try_with_env_or_str()

## [0.6.12] 2018-2-07

Add ReconfigurationHandle::parse_new_spec()

## [0.6.11] 2017-12-29

Fix README.md

## [0.6.10] 2017-12-29

Publish version based on log 0.4

## (...)

## [0.6.0] 2017-07-13

Use builder pattern for LogSpecification and Logger

- deprecate outdated API
- "objectify" LogSpecification
- improve documentation, e.g. document the dash/underscore issue
