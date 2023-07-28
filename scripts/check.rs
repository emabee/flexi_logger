#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! yansi = "0.5"
//! ```
extern crate yansi;
use std::process::Command;

macro_rules! run_command {
    ($cmd:expr , $($arg:expr),*) => (
        let mut command = command!($cmd, $($arg),*);
        let mut child = command.spawn().unwrap();
        let status = child.wait().unwrap();
        if !status.success() {
            print!("> {}",yansi::Paint::red("qualify terminates due to error"));
            std::process::exit(-1);
        }
    )
}

macro_rules! command {
    ($cmd:expr , $($arg:expr),*) => (
        {
            print!("\n> {}",yansi::Paint::yellow($cmd));
            let mut command = Command::new($cmd);
            $(
                print!(" {}",yansi::Paint::yellow(&$arg));
                command.arg($arg);
            )*
            print!("\n");
            command
        }
    )
}

// fn run_script(s: &str) {
//     let mut path = std::path::PathBuf::from(std::env::var("CARGO_SCRIPT_BASE_PATH").unwrap());
//     path.push(s);
//     let script = path.to_string_lossy().to_owned().to_string();
//     run_command!("cargo", "script", script);
// }

fn main() {
    // Check in important variants
    run_command!("cargo", "check");
    run_command!("cargo", "check", "--all-features");
    run_command!("cargo", "check", "--no-default-features");
    run_command!("cargo", "check", "--features= specfile");
    run_command!("cargo", "check", "--features= trc");

    // Clippy in important variants
    run_command!("cargo", "clippy", "--all-features", "--", "-D", "warnings");

    // doc
    #[rustfmt::skip]
    run_command!("cargo", "+nightly", "doc", "--all-features", "--no-deps", "--open");

    // say goodbye
    println!("\n> checks are done :-)  Looks like you're ready to do the full qualification?");
}
