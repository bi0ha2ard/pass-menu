use crate::gui::AppResult;

mod gui;
mod painting;
mod man;

fn main() -> crate::man::Result<()> {
    let listing = man::get_manpages()?;
    let pages = listing.get_entries();

    let res = gui::run_app("man page", &pages, true);

    if let AppResult::Match(e) = res {
        return e.show();
    }
    Ok(())
}
