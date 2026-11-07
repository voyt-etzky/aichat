#![allow(clippy::double_must_use)]

use std::env;
use std::process::{exit, Command};

fn main() {
    let aichat_name = if cfg!(windows) {
        "aichat.exe"
    } else {
        "aichat"
    };

    let mut cmd = if let Ok(current) = env::current_exe() {
        let candidate = current.parent().map(|p| p.join(aichat_name));
        if let Some(candidate) = candidate.filter(|p| p.is_file()) {
            Command::new(candidate)
        } else {
            Command::new(aichat_name)
        }
    } else {
        Command::new(aichat_name)
    };

    cmd.env("AICHAT_IS_AIQ", "1");
    cmd.args(env::args_os().skip(1));

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
            Ok(status) => exit(status.code().unwrap_or(1)),
            Err(err) => {
                eprintln!("Failed to execute aichat: {err}");
                exit(1);
            }
        }
    }
}
