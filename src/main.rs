use std::env;

use softbuffer::Context;
use winit::event_loop::{self, EventLoop};

use crate::pass::{Config, Result, find_pw_store, get_entries};

mod gui;
mod painting;
mod pass;

enum Action {
    User,
    Pass,
    AutoType,
    OpenUrl,
}

fn main() -> Result<()> {
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

    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(event_loop::ControlFlow::Wait);
    let context = Context::new(event_loop.owned_display_handle()).expect("display handle");

    let mut app = gui::App::new(context, prompt, &entries);

    event_loop.run_app(&mut app).expect("run");

    let config = Config::default();
    if let Some(e) = app.get_selected_entry() {
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
