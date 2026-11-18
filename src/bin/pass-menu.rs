use std::env;

use pass_menu::{
    gui::AppResult,
    notify,
    pass::{self, Config, find_pw_store, get_entries},
};

enum Action {
    User,
    TypeUser,
    TypePass,
    Pass,
    AutoType,
    OpenUrl,
}

fn help() {
    println!(
        r#"Usage: pass-menu <command>

Commands:
    autotype    Types out <user><TAB><password>
    typeuser    Types out <user>
    typepass    Types out <password>
    user        Copies <user> to clipboard
    pass        Copies <password> to clipboard
    open        Opens <url> field with xdg-open
    help        Shows this help
"#
    );
}

fn main() -> pass::Result<()> {
    let (command, prompt) = if let Some(arg) = env::args().nth(1) {
        match arg.as_str() {
            "user" => (Action::User, "Copy user"),
            "typeuser" => (Action::TypeUser, "Type user"),
            "typepass" => (Action::TypePass, "Type password"),
            "pass" => (Action::Pass, "Copy password"),
            "autotype" => (Action::AutoType, "Autotype"),
            "open" => (Action::OpenUrl, "Open URL"),
            "help" | "--help" | "-h" => {
                help();
                return Ok(());
            }
            _ => {
                eprintln!("Invalid argument '{arg}'");
                help();
                return Err(pass::PassError::ArgFail);
            }
        }
    } else {
        eprintln!("Action required");
        help();
        return Err(pass::PassError::ArgFail);
    };

    let pw_dir = find_pw_store()?;
    let mut entries = get_entries(&pw_dir)?;
    entries.sort_unstable();
    entries.dedup();

    let res = pass_menu::gui::run_app(prompt, &entries, false);

    let config = Config::default();
    if let AppResult::Match(e) = res {
        let worked = match command {
            Action::User => e.copy_user(&config),
            Action::TypeUser => e.type_user(&config),
            Action::TypePass => e.type_pass(&config),
            Action::Pass => e.copy_pass(&config),
            Action::AutoType => e.auto_type(&config),
            Action::OpenUrl => e.open_url(&config),
        };
        return worked.inspect_err(|err| {
            println!("{err:?}");
            match err {
                pass::PassError::StoreDiscoveryError(_) => {}
                pass::PassError::EnvironmentError => {}
                pass::PassError::PassError(error) => {
                    let _ = notify::notify("pass-menu", "pass failed", &error.to_string());
                }
                pass::PassError::ClipError(error) => {
                    let _ = notify::notify("pass-menu", "clip failed", &error.to_string());
                }
                pass::PassError::AutoTypeError(error) => {
                    let _ = notify::notify("pass-menu", "Auto Type", &error.to_string());
                }
                pass::PassError::UrlOpenError(error) => {
                    let _ = notify::notify("pass-menu", "URL Open", &error.to_string());
                }
                pass::PassError::IOError => {
                    let _ = notify::notify("pass-menu", "IO Error", &err.to_string());
                }
                pass::PassError::ProcessExited(proc, _) => match proc {
                    &"pass" => {}
                    _ => {
                        let _ = notify::notify(
                            "pass-menu",
                            &format!("{proc} failed"),
                            &err.to_string(),
                        );
                    }
                },
                pass::PassError::DecodeFail => {}
                pass::PassError::ParseFail(e) => {
                    let _ = notify::notify("pass-menu", "Parse failed", &e);
                }
                pass::PassError::ArgFail => {}
                pass::PassError::XsetError(_) => {}
            }
        });
    }
    Ok(())
}
