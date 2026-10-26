use std::{
    io::Write,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

#[derive(Debug)]
pub struct Entry {
    pub name: String,
    path: PathBuf,
}

pub struct Config {
    pass_binary: String,
    clip_binary: String,
    clip_args: Vec<String>,
    do_tool_binary: String,
    do_tool_type_args: Vec<String>,
    do_tool_tab_args: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pass_binary: "pass".to_string(),
            clip_binary: "xclip".to_string(),
            clip_args: vec![
                "-selection".to_string(),
                "clipboard".to_string(),
                "-rmlastnl".to_string(),
                "-wait".to_string(),
                "45000".to_string(),
            ],
            do_tool_binary: "xdotool".to_string(),
            do_tool_type_args: vec!["type".to_string(), "--clearmodifiers".to_string()],
            do_tool_tab_args: vec![
                "key".to_string(),
                "--clearmodifiers".to_string(),
                "Tab".to_string(),
            ],
        }
    }
}

const GPG_EXT: &str = ".gpg";
const USER_FIELD: &str = "user:";

fn field_from_stdout(stdout: &[u8], field: &str) -> Result<String> {
    for line in stdout.split(|c| *c == b'\n').skip(2) {
        let line = str::from_utf8(line)
            .map_err(|_| PassError::DecodeFail)?
            .trim();
        let user = line.strip_prefix(field);
        if let Some(user) = user {
            let user = user.trim();
            if user.is_empty() {
                return Err(PassError::ParseFail(format!("Empty '{field}' entry")));
            }
            return Ok(user.to_string());
        }
    }
    Err(PassError::ParseFail(
        "Field '{field}' not found".to_string(),
    ))
}

fn pass_from_stdout(stdout: &[u8]) -> Result<String> {
    stdout
        .split(|c| *c == b'\n')
        .next()
        .ok_or(PassError::ParseFail("No password".to_string()))
        .and_then(|line| str::from_utf8(line).map_err(|_| PassError::DecodeFail))
        .map(String::from)
}

pub fn do_type(text: &str, config: &Config) -> Result<()> {
    let mut cmd = std::process::Command::new(&config.do_tool_binary);
    cmd.args(&config.do_tool_type_args)
        .arg(text)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let mut handle = cmd.spawn().map_err(PassError::from_clip_spawn)?;
    handle
        .stdin
        .take()
        .ok_or(PassError::IOError)?
        .write_all(text.as_bytes())
        .map_err(|_| PassError::IOError)?;

    let res = handle.wait().map_err(PassError::from_autotype_spawn)?;
    if !res.success() {
        return Err(PassError::ProcessExited(res.code().unwrap_or_default()));
    }
    Ok(())
}

pub fn do_tab(config: &Config) -> Result<()> {
    let mut cmd = std::process::Command::new(&config.do_tool_binary);
    cmd.args(&config.do_tool_tab_args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let res = cmd
        .spawn()
        .map_err(PassError::from_autotype_spawn)?
        .wait()
        .map_err(PassError::from_autotype_spawn)?;
    if !res.success() {
        return Err(PassError::ProcessExited(res.code().unwrap_or_default()));
    }
    Ok(())
}

impl Entry {
    pub fn copy_pass(&self, config: &Config) -> Result<()> {
        let mut cmd = std::process::Command::new(&config.pass_binary);
        cmd.arg("show")
            .arg("--clip")
            .arg(&self.name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let res = cmd
            .spawn()
            .map_err(PassError::from_pass_spawn)?
            .wait()
            .map_err(PassError::from_pass_spawn)?;
        if !res.success() {
            return Err(PassError::ProcessExited(res.code().unwrap_or_default()));
        }
        Ok(())
    }

    fn get_pass_stdout(&self, config: &Config) -> Result<Vec<u8>> {
        let mut cmd = std::process::Command::new(&config.pass_binary);
        cmd.arg("show")
            .arg(&self.name)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        let res = cmd
            .spawn()
            .map_err(PassError::from_pass_spawn)?
            .wait_with_output()
            .map_err(PassError::from_pass_spawn)?;
        if !res.status.success() {
            return Err(PassError::ProcessExited(
                res.status.code().unwrap_or_default(),
            ));
        }
        Ok(res.stdout)
    }

    fn get_user(&self, config: &Config) -> Result<String> {
        let stdout = self.get_pass_stdout(config)?;
        field_from_stdout(&stdout, USER_FIELD)
    }

    pub fn copy_user(&self, config: &Config) -> Result<()> {
        let user = self.get_user(config)?;

        let mut cmd = std::process::Command::new(&config.clip_binary);
        cmd.args(&config.clip_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        let mut handle = cmd.spawn().map_err(PassError::from_clip_spawn)?;
        handle
            .stdin
            .take()
            .ok_or(PassError::IOError)?
            .write_all(user.as_bytes())
            .map_err(|_| PassError::IOError)?;

        handle.wait().map_err(PassError::from_clip_spawn)?;
        Ok(())
    }

    pub fn auto_type(&self, config: &Config) -> Result<()> {
        let stdout = self.get_pass_stdout(config)?;
        let pass = pass_from_stdout(&stdout)?;
        let user = field_from_stdout(&stdout, USER_FIELD)?;

        // TODO: potentially disable auto-repeat with xset?

        sleep(Duration::from_secs(1));
        do_type(&user, config)?;
        do_tab(config)?;
        sleep(Duration::from_millis(100));
        do_type(&pass, config)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PassError {
    #[error("Password store could not be resolved")]
    StoreDiscoveryError(std::io::Error),

    #[error("Could not discover basic environment")]
    EnvironmentError,

    #[error("Could not run pass")]
    PassError(std::io::Error),

    #[error("Clip failed")]
    ClipError(std::io::Error),

    #[error("Autotype failed")]
    AutoTypeError(std::io::Error),

    #[error("Could not send data to child process")]
    IOError,

    #[error("Pass exited non-zero")]
    ProcessExited(i32),

    #[error("Could not parse as UTF-8")]
    DecodeFail,

    #[error("Could not extract field")]
    ParseFail(String),
}

impl PassError {
    fn from_discovery(err: std::io::Error) -> Self {
        Self::StoreDiscoveryError(err)
    }

    fn from_pass_spawn(err: std::io::Error) -> Self {
        Self::PassError(err)
    }

    fn from_clip_spawn(err: std::io::Error) -> Self {
        Self::ClipError(err)
    }

    fn from_autotype_spawn(err: std::io::Error) -> Self {
        Self::AutoTypeError(err)
    }
}

type Result<T> = std::result::Result<T, PassError>;

fn extract_entry(path: PathBuf, prefix: &str) -> Option<Entry> {
    if !path.is_file() {
        return None;
    }
    let file_name = path.file_name()?;
    let file_name = file_name.to_string_lossy();
    let stripped = file_name.strip_suffix(GPG_EXT)?;
    let name = if prefix.is_empty() {
        stripped.to_string()
    } else {
        format!("{prefix}/{stripped}")
    };
    Some(Entry { name, path })
}

fn get_entries_impl(entries: &mut Vec<Entry>, password_store: &Path, prefix: &str) -> Result<()> {
    assert!(password_store.is_dir());
    for entry in password_store
        .read_dir()
        .map_err(PassError::from_discovery)?
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            let suffix = path.file_name().unwrap_or_default().display();
            let new_prefix = if prefix.is_empty() {
                suffix.to_string()
            } else {
                format!("{prefix}/{suffix}")
            };
            get_entries_impl(entries, &path, &new_prefix)?;
        } else if let Some(entry) = extract_entry(path, prefix) {
            entries.push(entry);
        }
    }
    Ok(())
}

pub fn get_entries(password_store: &Path) -> Result<Vec<Entry>> {
    if !password_store.is_dir() {
        return Err(PassError::StoreDiscoveryError(std::io::Error::from(
            std::io::ErrorKind::NotADirectory,
        )));
    }
    let mut res = Vec::new();
    get_entries_impl(&mut res, password_store, "")?;
    Ok(res)
}

pub fn find_pw_store() -> Result<PathBuf> {
    let store = std::env::var("PASSWORD_STORE_DIR")
        .map(PathBuf::from)
        .or_else(|_| {
            std::env::home_dir()
                .ok_or(PassError::EnvironmentError)
                .map(|mut home| {
                    home.push(".password-store");
                    home
                })
        })
        .and_then(|p| p.canonicalize().map_err(PassError::from_discovery));
    if store.as_ref().is_ok_and(|p| !p.is_dir()) {
        Err(PassError::StoreDiscoveryError(std::io::Error::from(
            std::io::ErrorKind::NotADirectory,
        )))
    } else {
        store
    }
}
