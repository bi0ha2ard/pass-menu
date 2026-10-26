use crate::pass::{Config, find_pw_store, get_entries};

mod pass;

fn main() {
    let config = Config::default();
    let pw_dir = find_pw_store().unwrap();
    let entries = get_entries(&pw_dir).unwrap();

    let f = entries.iter().find(|e| e.name == "net/wifi_guest").unwrap();
    // f.copy_user(&config).unwrap();
    f.auto_type(&config);
}
