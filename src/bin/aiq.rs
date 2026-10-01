#![allow(clippy::double_must_use)]

use std::env;
use std::process::{exit, Command};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut cmd = if let Ok(current) = env::current_exe() {
        let parent = current.parent();
        let aichat_name = if cfg!(windows) {
            "aichat.exe"
        } else {
            "aichat"
        };
        let candidate = parent.map(|p| p.join(aichat_name));
        if let Some(candidate) = candidate.filter(|p| p.exists()) {
            Command::new(candidate)
        } else {
            Command::new("aichat")
        }
    } else {
        Command::new("aichat")
    };

    cmd.arg("-r").arg("quick").args(&args);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        eprintln!("Failed to execute aichat: {err}");
        exit(1);
    }

    #[cfg(not(unix))]
    {
        match cmd.status() {
            Ok(status) => exit(status.code().unwrap_or(0)),
            Err(err) => {
                eprintln!("Failed to execute aichat: {err}");
                exit(1);
            }
        }
    }
}
