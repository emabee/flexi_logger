# flexi_logger

**A flexible and easy-to-use logger that writes logs to stderr and/or to files, and/or to
other output streams, and that can be influenced while the program is running.**

[![Latest version](https://img.shields.io/crates/v/flexi_logger.svg)](https://crates.io/crates/flexi_logger)
[![Documentation](https://docs.rs/flexi_logger/badge.svg)](https://docs.rs/flexi_logger)
[![License](https://img.shields.io/crates/l/flexi_logger.svg)](https://github.com/emabee/flexi_logger)
[![Build](https://img.shields.io/github/actions/workflow/status/emabee/flexi_logger/ci_test.yml?branch=master)](https://github.com/emabee/flexi_logger/actions?query=workflow%3ACI)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

## Usage

Add `flexi_logger` and `log` to the dependencies section in your project's `Cargo.toml`
(`log` is needed because `flexi_logger` plugs into the standard Rust logging facade given
by the [log crate](https://crates.io/crates/log),
and you use the ```log``` macros to write log lines from your code):

```toml
[dependencies]
flexi_logger = "0.28"
log = "0.4"
```

To provide the log specification via env variable `RUST_LOG` and get the log written to stderr,
add to an early place in your main:

```rust
flexi_logger::init();
```

Or, to provide a default log spec programmatically, use

```rust
flexi_logger::Logger::try_with_env_or_str("info, my::critical::module=trace")?.start()?;
```

or, to get the log e.g. written with high performance to a file,

```rust
use flexi_logger::{FileSpec, Logger, WriteMode};

let _logger = Logger::try_with_str("info, my::critical::module=trace")?
    .log_to_file(FileSpec::default())
    .write_mode(WriteMode::BufferAndFlush)
    .start()?;
```

There are many more configuration options to e.g.

* decide whether you want to write your logs to stdout or to a file,
* configure the path and the filenames of the log files,
* use file rotation,
* specify the line format for the log lines,
* apply a stateful filter before log lines are really written,
* define additional log streams, e.g for alert or security messages,
* support changing the log specification on the fly, while the program is running.

See

* the documentation of module
  [code_examples](https://docs.rs/flexi_logger/latest/flexi_logger/code_examples/index.html)
  for a bunch of examples,
* the [API documentation](https://docs.rs/flexi_logger/latest/flexi_logger)
  for a complete reference.

## Minimal rust version

The minimal supported rust version is currently "1.70.0".

## Crate Features

Make use of the non-default features by specifying them in your `Cargo.toml`, e.g.

```toml
[dependencies]
flexi_logger = { version = "0.28", features = ["async", "specfile", "compress"] }
log = "0.4"
```

or, to get the smallest footprint (and no colors), switch off even the default features:

```toml
[dependencies]
flexi_logger = { version = "0.28", default_features = false }
log = "0.4"
```

### **`async`**

Adds an additional write mode that decouples `flexi_logger`'s I/O from your application threads.
Works with `log_to_stdout()`, `log_to_stderr()`, and `log_to_file()`.
See [here](./docs/diagrams.pdf) for a performance comparison of some write modes.

Adds dependencies to
[`crossbeam-channel`](https://docs.rs/crossbeam-channel/latest/crossbeam_channel/)
and [`crossbeam-queue`](https://docs.rs/crossbeam-queue/latest/crossbeam_queue/).

### **`colors`** (*default feature*)

Getting colored output is also possible without this feature,
by implementing and using your own coloring format function.

The default feature `colors` simplifies this by doing three things:

* it activates the optional dependency to `nu_ansi_term` and
* provides additional colored pendants to the existing uncolored format functions
* it uses `colored_default_format()` for the output to stderr,
  and the non-colored `default_format()` for the output to files
* it switches off coloring if the output is not sent to a terminal but e.g. piped to another program.

**<span style="color:red">C</span><span style="color:blue">o</span><span
style="color:green">l</span><span style="color:orange">o</span><span
style="color:magenta">r</span><span style="color:darkturquoise">s</span>**,
or styles in general, are a matter of taste, and no choice will fit every need.
So you can override the default formatting and coloring in various ways.

With switching off the default features
(see [usage](#usage)) you can remove the `nu_ansi_term`-based coloring
but keep the capability to switch off your own coloring.

### **`compress`**

Adds two variants to the `enum` `Logger::Cleanup`, which allow keeping some
or all rotated log files in compressed form (`.gz`) rather than as plain text files.

### **`dont_minimize_extra_stacks`**

Normally, `flexi_logger` reduces the stack size of all threads that it might spawn
(flusher, specfile-watcher, async writer, cleanup) to a bare minimum.
For usecases where this is not desirable
(see [here](https://github.com/emabee/flexi_logger/issues/95) for some motivation),
you can activate this feature.

### **`json`**

Adds an additional format function `json_format` that prints the whole log line in json format,
like this:

```text
{"level":"WARN","timestamp":"2024-03-14 10:04:57.299908 +01:00","thread":"XY","module_path":"test_json","file":"src/test_json.rs","line":32,"text":"More foo than bar."}
```

Adds dependencies to `serde`, `serde_derive`, `serde_json`.

### **`kv`**

If you use the `kv` feature of the `log` crate to enrich the log-macro calls with key-value pairs,
then you should also use the `kv` feature of `flexi_logger`
so that these key-value pairs are also written by the
provided [format functions](https://docs.rs/flexi_logger/latest/flexi_logger/#functions).

### **`specfile`**

Adds a method `Logger::start_with_specfile(specfile)`.

If started with this method, `flexi_logger` uses the log specification
that was given to the factory method (one of `Logger::with...()`) as initial spec
and then tries to read the log specification from the named file.

If the file does not exist, it is created and filled with the initial spec.

By editing the log specification in the file while the program is running,
you can change the logging behavior in real-time.

The implementation of this feature uses some additional crates that you might
not want to depend on with your program if you don't use this functionality.
For that reason the feature is not active by default.

### **`specfile_without_notification`**

Pretty much like `specfile`, except that updates to the file are being ignored.
See [here](https://github.com/emabee/flexi_logger/issues/59) for more details.

### **`syslog_writer`**

Adds `SyslogWriter`, a `LogWriter` implementation that sends log entries to the syslog.

### **`textfilter`** (*default feature*)

Adds the ability to filter logs by text, but also adds a dependency on the regex crate.

### **`trc`**

An experimental feature that allows using `flexi_logger` functionality with `tracing`.

## Versions

See the [change log](https://github.com/emabee/flexi_logger/blob/master/CHANGELOG.md)
for more details.
