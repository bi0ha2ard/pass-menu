#[derive(Debug)]
pub struct ManEntry<'a> {
    title: &'a str,
    section: &'a str,
    summary: &'a str,
    repr: &'a str,
}

impl<'a> ManEntry<'a> {
    pub fn show(&self) -> Result<()> {
        let section = self.section.trim_start_matches('(').trim_end_matches(')');

        let mut cmd = std::process::Command::new(option_env!("CUSTOM_TERMINAL").unwrap_or("st"));
        cmd.args(["-e", "man", section, self.title])
            .env("MANPAGER", "nvim +Man!")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let res = cmd
            .spawn()
            .map_err(ManError::from_man)?
            .wait()
            .map_err(ManError::from_man)?;
        if !res.success() {
            return Err(ManError::ProcessExited(
                "man",
                res.code().unwrap_or_default(),
            ));
        }
        Ok(())
    }
}

impl AsRef<str> for ManEntry<'_> {
    fn as_ref(&self) -> &str {
        self.repr
    }
}

pub struct ManListing {
    buffer: Vec<u8>,
}

impl ManListing {
    pub fn get_entries(&self) -> Vec<ManEntry<'_>> {
        self.buffer
            .split(|c| *c == b'\n')
            .filter_map(parse_entry)
            .collect()
    }
}

fn parse_entry(raw_line: &'_ [u8]) -> Option<ManEntry<'_>> {
    let line = str::from_utf8(raw_line).ok()?;
    let mut iter = line.split_whitespace();
    let title = iter.next()?;
    let section = iter.next()?;
    let repr = {
        let offset = title.as_ptr() as usize - line.as_ptr() as usize;
        let end = (section.as_ptr() as usize + section.len()) - line.as_ptr() as usize;
        &line[offset..end]
    };
    let summary = {
        if let Some(sum_start) = iter.nth(1) {
            let offset = sum_start.as_ptr() as usize - line.as_ptr() as usize;
            line[offset..].trim_end()
        } else {
            ""
        }
    };
    Some(ManEntry {
        title,
        section,
        summary,
        repr,
    })
}

#[derive(thiserror::Error, Debug)]
pub enum ManError {
    #[error("Could not get manpage list")]
    DiscoveryError(std::io::Error),

    #[error("Could not show manpage")]
    ManError(std::io::Error),

    #[error("Process exited nonzero")]
    ProcessExited(&'static str, i32),
}

impl ManError {
    fn from_whatis_spawn(err: std::io::Error) -> Self {
        Self::DiscoveryError(err)
    }

    fn from_man(err: std::io::Error) -> Self {
        Self::ManError(err)
    }
}

pub type Result<T> = std::result::Result<T, ManError>;

pub fn get_manpages() -> Result<ManListing> {
    let mut cmd = std::process::Command::new("whatis");
    cmd.arg("-w")
        .arg("*")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let res = cmd
        .spawn()
        .map_err(ManError::from_whatis_spawn)?
        .wait_with_output()
        .map_err(ManError::from_whatis_spawn)?;
    if !res.status.success() {
        return Err(ManError::ProcessExited(
            "whatis",
            res.status.code().unwrap_or_default(),
        ));
    }
    Ok(ManListing { buffer: res.stdout })
}
