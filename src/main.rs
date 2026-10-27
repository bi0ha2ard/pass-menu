use winit::event_loop::{self, EventLoop};

use crate::pass::{Config, find_pw_store, get_entries};

mod pass;
mod gui;

fn main() {
    let config = Config::default();
    let pw_dir = find_pw_store().unwrap();
    let mut entries = get_entries(&pw_dir).unwrap();
    entries.sort_unstable();
    entries.dedup();

    // for e in &entries {
    //     println!("{}", e.name);
    // }

    let mut matcher = frizbee::Matcher::new("wifi", &frizbee::Config::default());
    let res = matcher.match_list(&entries);

    for m in res {
        println!("{m:?}: {:?}", entries.get(m.index as usize).expect("in bounds"));
    }


    // f.copy_user(&config).unwrap();
    // f.auto_type(&config).unwrap();
    //
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(event_loop::ControlFlow::Wait);

    let mut app = gui::App::default();
    event_loop.run_app(&mut app).expect("run");
}
