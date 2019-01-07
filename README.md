# flexi_logger

A flexible and easy-to-use logger that writes logs to stderr and/or to files.

## Usage

Add flexi_logger to the dependencies section in your project's `Cargo.toml`, with

```toml
[dependencies]
flexi_logger = "^0.10.4"
log = "0.4"
```

or, if you want to use the `specfile` feature, with

```toml
[dependencies]
flexi_logger = { version = "^0.10.4", features = ["specfile"] }
log = "0.4"
```

Note: `log` is needed because `flexi_logger` plugs into the standard Rust logging facade given
by the [log crate](https://crates.io/crates/log),
and you use the ```log``` macros to write log lines from your code.

### Example 1

To read the log specification from the environment variable  `RUST_LOG` and write the logs
to stderr (i.e., behave like `env_logger`),
do this early in your program:

```rust
use flexi_logger::Logger;
// ...
Logger::with_env()
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
```

After that, you just use the log-macros from the log crate.

To log differently, you may

* choose an alternative `with...` method,
* and/or add some configuration options,
* and/or choose an alternative `start...` method.

### Example 2

In the folllowing example we

* provide the loglevel-specification programmatically, as String, while still allowing it
   to be overridden by the environment variable `RUST_LOG`,
* and we configure `flexi_logger` to write into a log file in folder `log_files`,
* and write the log entries with time and location info (`opt_format`)

```rust
use flexi_logger::{Logger,opt_format};
// ...
Logger::with_env_or_str("myprog=debug, mylib=warn")
            .log_to_file()
            .directory("log_files")
            .format(opt_format)
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
```

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

## Versions

See the [change log](https://github.com/emabee/flexi_logger/blob/master/CHANGELOG.md).
