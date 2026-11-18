use std::{
    io::Write,
    path::{Path, PathBuf},
    process::ExitStatus,
    thread::sleep,
    time::Duration,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    pub name: String,
}

impl AsRef<str> for Entry {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

pub struct Config {
    pass_binary: String,
    clip_binary: String,
    clip_args: Vec<String>,
    do_tool_binary: String,
    do_tool_type_args: Vec<String>,
    do_tool_tab_args: Vec<String>,
    open_tool: String,
}

struct RepeatToggle {
    must_reenable: bool,
}

const AUTO_REPAET_PREFIX: &str = "auto repeat:";

fn parse_repeat_rate(stdout: &[u8]) -> bool {
    let Ok(text) = str::from_utf8(stdout) else {
        return false;
    };
    let Some(offs) = text.find(AUTO_REPAET_PREFIX) else {
        return false;
    };
    let tail = text[offs + AUTO_REPAET_PREFIX.len()..].trim_start();
    tail.starts_with("on")
}

#[cfg(feature = "x11")]
fn try_disable_kb_repeat() -> Result<bool> {
    let mut cmd = std::process::Command::new("xset");
    cmd.arg("q")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let res = cmd
        .spawn()
        .map_err(PassError::from_xset_spawn)?
        .wait_with_output()
        .map_err(PassError::from_xset_spawn)?;
    if !res.status.success() {
        return Err(PassError::ProcessExited(
            "xset",
            res.status.code().unwrap_or_default(),
        ));
    }
    let enabled = parse_repeat_rate(&res.stdout);
    if enabled {
        let _ = set_kb_repeat(false);
    }
    Ok(enabled)
}

#[cfg(feature = "x11")]
fn set_kb_repeat(enabled: bool) -> Result<ExitStatus> {
    let mut cmd = std::process::Command::new("xset");
    cmd.arg("r")
        .arg(if enabled { "on" } else { "off" })
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    cmd.spawn()
        .map_err(PassError::from_xset_spawn)?
        .wait()
        .map_err(PassError::from_xset_spawn)
}

impl RepeatToggle {
    fn new() -> Self {
        Self {
            must_reenable: cfg_select! {
                feature = "x11" => {try_disable_kb_repeat().unwrap_or_default()}
                feature = "wayland" => {false}
            },
        }
    }
}

#[cfg(feature = "x11")]
impl Drop for RepeatToggle {
    fn drop(&mut self) {
        if !self.must_reenable {
            return;
        }
        let _ = set_kb_repeat(true);
    }
}

impl Default for Config {
    fn default() -> Self {
        let (clip_binary, clip_args) = cfg_select! {
            feature = "x11" => ("xclip".to_string(),
            vec![
            "-selection".to_string(),
            "clipboard".to_string(),
            "-rmlastnl".to_string(),
            "-loops".to_string(),
            "1".to_string(), // Can break on xfce4 because xfsettingsd takes one clipboard loop...
            "-in".to_string(),
            ]),
            feature = "wayland" => ("wl-copy".to_string(),
            vec![
            "--trim-newline".to_string(),
            "--paste-once".to_string(),
            "--sensitive".to_string(),
            ])
        };
        let (do_tool_binary, do_tool_type_args, do_tool_tab_args) = cfg_select! {
            feature = "x11" => ("xdotool".to_string(),
            vec!["type".to_string(), "--clearmodifiers".to_string(), "--file".to_string(), "-".to_string()],
            vec![
            "key".to_string(),
            "--clearmodifiers".to_string(),
            "Tab".to_string(),
            ]),
            feature = "wayland" =>
                ("wtype".to_string(),
                vec![ "-d".to_string(), "16".to_string(), "-".to_string()],
                vec![
                "-k".to_string(),
                "Tab".to_string(),
                ])

        };
        Self {
            pass_binary: "pass".to_string(),
            clip_binary,
            clip_args,
            do_tool_binary,
            do_tool_type_args,
            do_tool_tab_args,
            open_tool: "xdg-open".to_string(),
        }
    }
}

const GPG_EXT: &str = ".gpg";
const USER_FIELD: &str = "user:";
const URL_FIELD: &str = "url:";
const AUTOTYPE_START_DELAY: Duration = Duration::from_millis(500);

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
    Err(PassError::ParseFail(format!("Field '{field}' not found")))
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
    assert!(!text.is_empty());
    assert!(text.len() < 1024);
    let mut cmd = std::process::Command::new(&config.do_tool_binary);
    cmd.args(&config.do_tool_type_args)
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
        return Err(PassError::ProcessExited(
            "dotool",
            res.code().unwrap_or_default(),
        ));
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
        return Err(PassError::ProcessExited(
            "dotool",
            res.code().unwrap_or_default(),
        ));
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
            return Err(PassError::ProcessExited(
                "pass",
                res.code().unwrap_or_default(),
            ));
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
                "pass",
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

    pub fn type_user(&self, config: &Config) -> Result<()> {
        let user = self.get_user(config)?;
        let guard = RepeatToggle::new();
        sleep(AUTOTYPE_START_DELAY);
        let res = do_type(&user, config);
        drop(guard);
        res
    }

    pub fn type_pass(&self, config: &Config) -> Result<()> {
        let stdout = self.get_pass_stdout(config)?;
        let pass = pass_from_stdout(&stdout)?;
        let guard = RepeatToggle::new();
        sleep(AUTOTYPE_START_DELAY);
        let res = do_type(&pass, config);
        drop(guard);
        res
    }

    pub fn auto_type(&self, config: &Config) -> Result<()> {
        let stdout = self.get_pass_stdout(config)?;
        let pass = pass_from_stdout(&stdout)?;
        let user = field_from_stdout(&stdout, USER_FIELD)?;

        let guard = RepeatToggle::new();

        sleep(AUTOTYPE_START_DELAY);
        do_type(&user, config)?;
        do_tab(config)?;
        sleep(Duration::from_millis(100));
        let res = do_type(&pass, config);
        drop(guard);
        res
    }

    pub fn open_url(&self, config: &Config) -> Result<()> {
        let stdout = self.get_pass_stdout(config)?;
        let url = field_from_stdout(&stdout, URL_FIELD)?;

        let mut cmd = std::process::Command::new(&config.open_tool);
        cmd.arg(&url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        let mut handle = cmd.spawn().map_err(PassError::from_url_open_spawn)?;
        handle.wait().map_err(PassError::from_url_open_spawn)?;
        Ok(())
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

    #[error("xset failed")]
    XsetError(std::io::Error),

    #[error("Autotype failed")]
    UrlOpenError(std::io::Error),

    #[error("Could not send data to child process")]
    IOError,

    #[error("Process exited non-zero")]
    ProcessExited(&'static str, i32),

    #[error("Could not parse as UTF-8")]
    DecodeFail,

    #[error("Could not extract field")]
    ParseFail(String),

    #[error("Invalid argument")]
    ArgFail,
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

    fn from_url_open_spawn(err: std::io::Error) -> Self {
        Self::UrlOpenError(err)
    }

    fn from_autotype_spawn(err: std::io::Error) -> Self {
        Self::AutoTypeError(err)
    }

    fn from_xset_spawn(err: std::io::Error) -> Self {
        Self::XsetError(err)
    }
}

pub type Result<T> = std::result::Result<T, PassError>;

fn extract_entry(path: &Path, prefix: &str) -> Option<Entry> {
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
    Some(Entry { name })
}

const MAX_RECURSION: u32 = 8;
const MAX_ENTRIES: usize = 1024;

fn get_entries_impl(
    entries: &mut Vec<Entry>,
    password_store: &Path,
    prefix: &str,
    depth: u32,
) -> Result<()> {
    assert!(password_store.is_dir());
    if depth > MAX_RECURSION {
        // or error?
        return Ok(());
    }
    for entry in password_store
        .read_dir()
        .map_err(PassError::from_discovery)?
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            let fname = path.file_name().unwrap_or_default();
            if fname.eq_ignore_ascii_case(".git") {
                continue;
            }
            let suffix = fname.display();
            let new_prefix = if prefix.is_empty() {
                suffix.to_string()
            } else {
                format!("{prefix}/{suffix}")
            };
            get_entries_impl(entries, &path, &new_prefix, depth + 1)?;
        } else if let Some(entry) = extract_entry(&path, prefix) {
            entries.push(entry);
            if entries.len() > MAX_ENTRIES {
                return Ok(());
            }
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
    get_entries_impl(&mut res, password_store, "", 0)?;
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
