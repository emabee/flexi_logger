#!/usr/bin/env run-cargo-script
//! ```cargo
//! [dependencies]
//! yansi = "1.0"
//! ```
extern crate yansi;
use std::{process::Command, time::Instant};

fn main() {
    macro_rules! run_command {
        ($cmd:expr) => {
            let mut command = command!($cmd);
            let mut child = command.spawn().unwrap();
            let status = child.wait().unwrap();
            if !status.success() {
                print!("> {}", yansi::Paint::red("qualify terminates due to error"));
                std::process::exit(-1);
            }
        };
    }

    macro_rules! command {
        ($cmd:expr) => {{
            println!("\n> {}", yansi::Paint::yellow($cmd));
            let mut chips = $cmd.split(' ');
            let mut command = Command::new(chips.next().unwrap());
            for chip in chips {
                command.arg(chip);
            }
            command
        }};
    }

    // works:
    run_command!("cargo build");

    // works:
    run_command!("cargo clippy -- -D warnings");

    // does not abort, but reports inappropriate warnings
    run_command!("cargo +nightly clippy");

    // aborts because of the inappropriate  warnings
    run_command!("cargo +nightly clippy -- -D warnings");
}
