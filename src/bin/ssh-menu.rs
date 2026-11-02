use std::env;

use pass_menu::{
    gui::AppResult,
    ssh::{SshTarget, get_ssh_hosts},
    ssh
};

fn main() -> ssh::Result<()> {
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

    let res = pass_menu::gui::run_app(command, &hosts, false);

    match res {
        AppResult::Match(h) => ssh::run_command(command, h),
        AppResult::Partial(p) => ssh::run_command(command, &SshTarget { host: p }),
        AppResult::Aborted => Ok(()),
    }
}
