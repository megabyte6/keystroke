use std::rc::Rc;

use anyhow::{Context, Result};
use slint::LogicalSize;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

slint::include_modules!();

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("caps=info,slint=warn")),
        )
        .init();

    info!("starting app");
    let ctx = Rc::new(AppContext::new()?);
    if let Err(err) = slint::set_xdg_app_id("caps") {
        warn!(error = %err, "failed to register XDG app ID");
    }
    ctx.impl_callbacks();
    ctx.windows
        .main
        .window()
        .set_size(LogicalSize::new(1000.0, 800.0));
    ctx.windows
        .settings
        .window()
        .set_size(LogicalSize::new(800.0, 600.0));

    info!("running main window");
    ctx.windows.main.run().context("slint platform crashed")?;

    info!("exiting app");
    Ok(())
}

fn save_settings_and_exit() {
    if let Err(err) = slint::quit_event_loop() {
        error!(error = %err, "quitting app resulted in error");
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
            info!("close requested");
            save_settings_and_exit();
            slint::CloseRequestResponse::HideWindow
        });

        self.windows
            .main
            .on_check_for_updates(|| warn!("check for updates not implemented yet"));

        let settings_weak = self.windows.settings.as_weak();
        self.windows.main.on_open_settings(move || {
            info!("opening settings window");
            if let Some(settings) = settings_weak.upgrade() {
                let window = settings.window();
                if let Err(err) = window.show() {
                    error!("failed to show settings window: {err}");
                    return;
                }
                // some backends don't schedule an initial paint when showing a window from a menu.
                // request_redraw() ensures the first frame actually gets rendered.
                window.request_redraw();
            }
        });

        self.windows.main.on_quit(|| {
            info!("quit requested");
            save_settings_and_exit();
        });

        self.windows
            .main
            .on_fetch_students(|| warn!("fetch students not implemented yet"));
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
