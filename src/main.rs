use crate::pass::{Config, find_pw_store, get_entries};

mod pass;

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


    let f = entries.iter().find(|e| e.name == "net/wifi_guest").unwrap();
    // f.copy_user(&config).unwrap();
    // f.auto_type(&config).unwrap();
}
