use std::env;

use crate::{
    gui::AppResult,
    ssh::{SshTarget, get_ssh_hosts},
};

mod gui;
mod painting;
mod ssh;

fn main() -> crate::ssh::Result<()> {
    let command = if let Some(arg) = env::args().nth(1) {
        match arg.as_str() {
            "ssh" => "ssh",
            "sftp" => "sftp",
            _ => {
                eprintln!("Invalid argument '{arg}', pass one of 'ssh', 'sftp'");
                return Err(ssh::SshError::ArgFail);
            }
        }
    } else {
        "ssh"
    };
    let mut hosts = get_ssh_hosts()?;
    hosts.sort_unstable();

    let res = gui::run_app(command, &hosts, false);

    match res {
        AppResult::Match(h) => ssh::run_command(command, h),
        AppResult::Partial(p) => ssh::run_command(command, &SshTarget { host: p }),
        AppResult::Aborted => Ok(()),
    }
}
