# flexi_logger
A flexible logger that can write to stderr or to log files

## Documentation
See https://docs.rs/flexi_logger/

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
flexi_logger = "0.6"
log = "0.4"
```

and this to your crate root:

```rust
#[macro_use]
extern crate log;
extern crate flexi_logger;
```

Early in the start-up of your program, call something like

```text
  use flexi_logger::Logger;

  Logger::with_str("modx::mody=info")
      // ... your configuration options go here ...
      .start()
      .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
```
