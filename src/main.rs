use anyhow::{Context, Result};

slint::include_modules!();

fn main() -> Result<()> {
    let main_window = init()?;

    main_window.run().context("slint platform crashed")?;

    Ok(())
}

fn init() -> Result<MainWindow> {
    let main_window = MainWindow::new().context("failed to create main window")?;
    Ok(main_window)
}
