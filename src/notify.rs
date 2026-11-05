pub fn notify(app_name: &str, title: &str, body: &str) -> std::io::Result<bool> {
    let mut cmd = std::process::Command::new("notify-send");
    cmd.args([
        "--expire-time=5000",
        "-a",
        app_name,
        "--transient",
        "--urgency=low",
        title,
        body,
    ])
    .stdin(std::process::Stdio::null())
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());
    cmd.spawn()?.wait().map(|e| e.success())
}
