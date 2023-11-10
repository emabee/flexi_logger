#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! yansi = "0.5"
//! ```
extern crate yansi;
use std::process::Command;

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
        print!("\n> {}\n", yansi::Paint::yellow($cmd));
        let mut chips = $cmd.split(' ');
        let mut command = Command::new(chips.next().unwrap());
        for chip in chips {
            command.arg(chip);
        }
        command
    }};
}

fn run_script(s: &str) {
    let mut path = std::path::PathBuf::from(std::env::var("CARGO_SCRIPT_BASE_PATH").unwrap());
    path.push(s);
    let command = format!(
        "cargo script {}",
        path.to_string_lossy().to_owned().to_string()
    );
    run_command!(&command);
}

fn main() {
    // Build in important variants
    run_command!("cargo build --release --all-features");

    // Clippy in important variants
    run_command!("cargo clippy --all-features -- -D warnings");

    // Run tests in important variants
    run_command!("cargo test --release --all-features");
    run_script("cleanup");

    // doc
    run_command!("cargo doc --all-features --no-deps --open");

    // say goodbye
    println!(
        "\n> fast qualification is done :-)  Looks like you're ready to do the full qualification?"
    );
}
