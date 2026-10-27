use softbuffer::Context;
use winit::event_loop::{self, EventLoop};

use crate::pass::{Config, find_pw_store, get_entries};

mod gui;
mod pass;
mod painting;

fn main() {
    let pw_dir = find_pw_store().unwrap();
    let mut entries = get_entries(&pw_dir).unwrap();
    entries.sort_unstable();
    entries.dedup();

    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(event_loop::ControlFlow::Wait);
    let context = Context::new(event_loop.owned_display_handle()).expect("display handle");

    let mut app = gui::App::new(context, entries);

    event_loop.run_app(&mut app).expect("run");

    let config = Config::default();
    if let Some(e) = app.get_selected_entry() {
        println!("{}", e.name);
        e.copy_user(&config).expect("copy");
    }
}
