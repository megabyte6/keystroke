use std::rc::Rc;

use anyhow::{Context, Result};
use slint::set_xdg_app_id;

slint::include_modules!();

fn main() -> Result<()> {
    let ctx = Rc::new(AppContext::new()?);
    set_xdg_app_id("caps").context("failed to register XDG app ID")?;
    ctx.impl_callbacks();

    ctx.windows.main.run().context("slint platform crashed")?;

    Ok(())
}

fn save_settings_and_exit() {
    if let Err(err) = slint::quit_event_loop() {
        eprintln!("quitting app resulted in error: {err}");
    }
}

struct AppContext {
    windows: AppWindows,
}

impl AppContext {
    fn new() -> Result<Self> {
        Ok(Self {
            windows: AppWindows::new()?,
        })
    }

    fn impl_callbacks(self: &Rc<Self>) {
        self.windows.main.window().on_close_requested(|| {
            save_settings_and_exit();
            slint::CloseRequestResponse::HideWindow
        });

        self.windows
            .main
            .on_check_for_updates(|| println!("check for updates not implemented yet"));

        let settings_weak = self.windows.settings.as_weak();
        self.windows.main.on_open_settings(move || {
            if let Some(settings) = settings_weak.upgrade() {
                let window = settings.window();
                if let Err(err) = window.show() {
                    eprintln!("failed to show settings window: {err}");
                    return;
                }
                // some backends don't schedule an initial paint when showing a window from a menu.
                // request_redraw() ensures the first frame actually gets rendered.
                window.request_redraw();
            }
        });

        self.windows.main.on_quit(|| {
            save_settings_and_exit();
        });

        self.windows
            .main
            .on_fetch_students(|| println!("fetch students not implemented yet"));
    }
}

struct AppWindows {
    main: MainWindow,
    settings: SettingsWindow,
}

impl AppWindows {
    fn new() -> Result<Self> {
        Ok(Self {
            main: MainWindow::new().context("failed to create main window")?,
            settings: SettingsWindow::new().context("failed to create settings window")?,
        })
    }
}
