use pass_menu::gui::AppResult;
use pass_menu::man;

fn main() -> man::Result<()> {
    let listing = man::get_manpages()?;
    let pages = listing.get_entries();

    let res = pass_menu::gui::run_app("man page", &pages, true);

    if let AppResult::Match(e) = res {
        return e.show();
    }
    Ok(())
}
