use anyhow::{Context, Result};
use slint::set_xdg_app_id;

slint::include_modules!();

fn main() -> Result<()> {
    let main_window = init()?;

    main_window.run().context("slint platform crashed")?;

    Ok(())
}

fn init() -> Result<MainWindow> {
    let main_window = MainWindow::new().context("failed to create main window")?;
    set_xdg_app_id("caps").context("failed to register XDG app ID")?;
    Ok(main_window)
}
