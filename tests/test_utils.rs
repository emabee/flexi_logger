const CTRL_INDEX: &str = "CTRL_INDEX";

// launch child process from same executable and sets there an additional environment variable
// or finds this environment variable and returns its value
pub fn dispatch(count: u8) -> Option<u8> {
    match std::env::var(CTRL_INDEX) {
        Err(_) => {
            println!("dispatcher");
            let progname = std::env::args().next().unwrap();
            let nocapture = std::env::args().find(|a| a == "--nocapture").is_some();
            for value in 0..count {
                let mut command = std::process::Command::new(progname.to_string());
                if nocapture {
                    command.arg("--nocapture");
                }
                let status = command
                    .env(CTRL_INDEX, value.to_string())
                    .status()
                    .expect("Command failed to start");
                assert!(status.success());
            }
            None
        }
        Ok(value) => {
            println!("executor {}", value);
            Some(value.parse().unwrap())
        }
    }
}
