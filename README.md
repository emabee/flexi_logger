# flexi_logger
A flexible logger that can write to stderr or to log files

## Documentation
See http://emabee.atwebpages.com/rust/flexi_logger/index.html


## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
flexi_logger = "0.5"
log = "*"
```

and this to your crate root:

```rust
#[macro_use]
extern crate log;
extern crate flexi_logger;
```
