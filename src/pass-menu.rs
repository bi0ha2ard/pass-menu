use std::env;

use crate::{
    gui::AppResult,
    pass::{Config, find_pw_store, get_entries},
};

mod gui;
mod painting;
mod pass;

enum Action {
    User,
    Pass,
    AutoType,
    OpenUrl,
}

fn main() -> crate::pass::Result<()> {
    let (command, prompt) = if let Some(arg) = env::args().nth(1) {
        match arg.as_str() {
            "user" => (Action::User, "Copy user"),
            "pass" => (Action::Pass, "Copy password"),
            "autotype" => (Action::AutoType, "Autotype"),
            "open" => (Action::OpenUrl, "Open URL"),
            _ => {
                eprintln!(
                    "Invalid argument '{arg}', pass one of 'user', 'pass', 'autotype', 'open'"
                );
                return Err(pass::PassError::ArgFail);
            }
        }
    } else {
        eprintln!("Action required, pass one of 'user', 'pass', 'autotype', 'open'");
        return Err(pass::PassError::ArgFail);
    };

    let pw_dir = find_pw_store()?;
    let mut entries = get_entries(&pw_dir)?;
    entries.sort_unstable();
    entries.dedup();

    let res = gui::run_app(prompt, &entries);

    let config = Config::default();
    if let AppResult::Match(e) = res {
        println!("{}", e.name);
        return match command {
            Action::User => e.copy_user(&config),
            Action::Pass => e.copy_pass(&config),
            Action::AutoType => e.auto_type(&config),
            Action::OpenUrl => e.open_url(&config),
        };
    }
    Ok(())
}
