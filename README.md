# flexi_logger

**A flexible and easy-to-use logger that writes logs to stderr and/or to files,
and that can be influenced while the program is running.**

## Usage

Add flexi_logger to the dependencies section in your project's `Cargo.toml`, with

```toml
[dependencies]
flexi_logger = "^0.11.3"
log = "0.4"
```

or, if you want to use the optional features, with

```toml
[dependencies]
flexi_logger = { version = "^0.11.3", features = ["specfile", "ziplogs"] }
log = "0.4"
```

Note: `log` is needed because `flexi_logger` plugs into the standard Rust logging facade given
by the [log crate](https://crates.io/crates/log),
and you use the ```log``` macros to write log lines from your code.

## Example 1: log to stderr

To read the log specification from the environment variable  `RUST_LOG` and write the logs
to stderr (i.e., behave like `env_logger`),
do this early in your program:

```rust
flexi_logger::Logger::with_env()
            .start()
            .unwrap();
```

After that, you just use the log-macros from the log crate.

To log differently, you may

* choose an alternative `with...` method,
* and/or add some configuration options,
* and/or choose an alternative `start...` method.

## Example 2: log to files in a folder

In the folllowing example we

* provide the loglevel-specification programmatically, as String, while still allowing it
   to be overridden by the environment variable `RUST_LOG`,
* and we configure `flexi_logger` to write into a log file in folder `log_files`,
* and write the log entries with time and location info (`opt_format`)

```rust
use flexi_logger::{Logger, opt_format};
// ...
Logger::with_env_or_str("myprog=debug, mylib=warn")
            .log_to_file()
            .directory("log_files")
            .format(opt_format)
            .start()
            .unwrap();
```

## Example 3: reconfigure the log-spec programmatically

Obtain the `ReconfigurationHandle` (using `.start()`):

```rust
let mut log_handle = flexi_logger::Logger::with_str("info")
    // ... logger configuration ...
    .start()
    .unwrap();
```

and modify the effective log specification from within your code:

```rust
// ...
log_handle.parse_and_push_temp_spec("info, critical_mod = trace");
// ... critical calls ...
log_handle.pop_temp_spec();
// ... continue with the log spec you had before.
```

## Example 4: reconfigure the log-spec dynamically by editing a spec-file

If you start  `flexi_logger` with a specfile, e.g.

```rust
flexi_logger::Logger::with_str("info")
   .start_with_specfile("/server/config/logspec.toml")
   .unwrap();
```

then you can change the logspec dynamically, *while your program is running*,
by editing the specfile.

See the API documentation of
[`Logger::start_with_specfile()`](https://docs.rs/flexi_logger/latest/flexi_logger/struct.Logger.html#method.start_with_specfile)
for more details.

## Options

There are configuration options to e.g.

* decide whether you want to write your logs to stderr or to a file,
* configure the path and the filenames of the log files,
* use file rotation,
* specify the line format for the log lines,
* define additional log streams, e.g for alert or security messages,
* support changing the log specification on the fly, while the program is running,

See the API documentation for a complete reference.

## Crate Features

### `specfile`

The `specfile` feature adds a method `Logger::start_with_specfile(specfile)`.

If started with this method, `flexi_logger` uses the log specification
that was given to the factory method (one of `Logger::with...()`) as initial spec
and then tries to read the log specification from the named file.

If the file does not exist, it is created and filled with the initial spec.

By editing the log specification in the file while the program is running,
you can change the logging behavior in real-time.

The implementation of this feature uses some additional crates that you might
not want to depend on with your program if you don't use this functionality.
For that reason the feature is not active by default.

### `ziplogs`

The `ziplogs` feature adds two options to the `Logger::Cleanup` `enum`, which allow keeping some
or all rotated log files in zipped form rather than as text files.

## Versions

See the [change log](https://github.com/emabee/flexi_logger/blob/master/CHANGELOG.md).
