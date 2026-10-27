use winit::event_loop::{self, EventLoop};

use crate::{pass::{Config, find_pw_store, get_entries}};

mod pass;
mod gui;

fn main() {
    let pw_dir = find_pw_store().unwrap();
    let mut entries = get_entries(&pw_dir).unwrap();
    entries.sort_unstable();
    entries.dedup();

    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(event_loop::ControlFlow::Wait);

    let mut app = gui::App::new(entries);

    let config = Config::default();
    event_loop.run_app(&mut app).expect("run");
    if let Some(e) = app.get_selected_entry() {
        println!("{}", e.name);
        e.copy_user(&config).expect("copy");
    }
}
