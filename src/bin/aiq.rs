#![allow(clippy::double_must_use)]

use std::env;
use std::process::{exit, Command};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let aichat_name = if cfg!(windows) {
        "aichat.exe"
    } else {
        "aichat"
    };

    let mut cmd = if let Ok(current) = env::current_exe() {
        let candidate = current.parent().map(|p| p.join(aichat_name));
        if let Some(candidate) = candidate.filter(|p| p.exists()) {
            Command::new(candidate)
        } else {
            Command::new(aichat_name)
        }
    } else {
        Command::new(aichat_name)
    };

    cmd.env("AICHAT_IS_AIQ", "1");
    if !has_role_or_mode_flag(&args) {
        cmd.arg("-r").arg("quick");
    }
    cmd.args(&args);

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

fn has_role_or_mode_flag(args: &[String]) -> bool {
    for arg in args {
        if arg == "--" {
            break;
        }
        if matches!(
            arg.as_str(),
            "-r" | "--role"
                | "-p"
                | "--prompt"
                | "-e"
                | "--execute"
                | "-c"
                | "--code"
                | "-a"
                | "--agent"
        ) || arg.starts_with("-r=")
            || arg.starts_with("--role=")
            || arg.starts_with("--prompt=")
            || arg.starts_with("--agent=")
        {
            return true;
        }
    }
    false
}
