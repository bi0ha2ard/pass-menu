use std::process::ExitStatus;

#[derive(thiserror::Error, Debug)]
pub enum SshError {
    #[error("SSH config could not be loaded")]
    ConfigLoadError(std::io::Error),

    #[error("Could not discover basic environment")]
    EnvironmentError,

    #[error("SSH config could not be parsed")]
    ConfigParseError(std::io::Error),

    #[error("Could not spawn process")]
    ExecError(std::io::Error),

    #[error("Pass exited non-zero")]
    ProcessExited(&'static str, i32),

    #[error("Invalid arguments")]
    ArgFail,
}

pub type Result<T> = std::result::Result<T, SshError>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SshTarget {
    pub host: String,
}

impl AsRef<str> for SshTarget {
    fn as_ref(&self) -> &str {
        &self.host
    }
}

fn extract_host(line: &str) -> Option<SshTarget> {
    let line = line.trim();
    let mut words = line.split_ascii_whitespace();
    let Some(command) = words.next() else {
        return None;
    };
    if command != "Host" && command != "host" {
        return None;
    }
    words.next().map(|h| SshTarget { host: h.to_owned() })
}

pub fn get_ssh_hosts() -> Result<Vec<SshTarget>> {
    let config_file = std::env::home_dir()
        .ok_or(SshError::EnvironmentError)
        .map(|mut home| {
            home.push(".ssh");
            home.push("config");
            home
        })
        .and_then(|p| p.canonicalize().map_err(|e| SshError::ConfigLoadError(e)))?;

    let ssh_config =
        std::fs::read_to_string(config_file).map_err(|e| SshError::ConfigLoadError(e))?;

    Ok(ssh_config
        .lines()
        .filter_map(extract_host)
        .collect::<Vec<SshTarget>>())
}

pub fn run_command(prog: &'static str, target: &SshTarget) -> Result<()> {
    let mut cmd = std::process::Command::new("st");
    cmd.arg("-e")
        .arg(prog)
        .arg(target.as_ref())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    // TODO: fork+exec to detach fully, but maybe it's not worth it.
    let mut handle = cmd.spawn().map_err(|e| SshError::ExecError(e))?;
    let ret = handle.wait().map_err(|e| SshError::ExecError(e))?;
    if !ret.success() {
        return Err(SshError::ProcessExited(
            prog,
            ret.code().unwrap_or_default(),
        ));
    }
    Ok(())
}
