use std::rc::Rc;

use anyhow::{Context, Result};
use slint::LogicalSize;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

slint::include_modules!();

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("caps=info,slint=warn")),
        )
        .init();

    debug!("starting app");
    let ctx = Rc::new(AppContext::new().unwrap_or_else(|err| {
        error!(error = %err, "failed to create app context");
        std::process::exit(1);
    }));
    slint::set_xdg_app_id("caps")
        .unwrap_or_else(|err| error!(error = %err, "failed to register XDG app ID"));
    debug!("implement UI callbacks");
    ctx.impl_callbacks();

    debug!("show main window");
    ctx.windows.main.run().unwrap_or_else(|err| {
        error!(error = %err, "slint platform crashed");
        std::process::exit(1);
    });

    debug!("exiting app");
}

fn save_settings_and_exit() {
    slint::quit_event_loop()
        .unwrap_or_else(|err| error!(error = %err, "error encountered while quitting app"));
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
            debug!("close requested");
            save_settings_and_exit();
            slint::CloseRequestResponse::HideWindow
        });

        self.windows
            .main
            .on_check_for_updates(|| warn!("check for updates not implemented yet"));

        let settings_weak = self.windows.settings.as_weak();
        self.windows.main.on_open_settings(move || {
            debug!("opening settings window");
            if let Some(settings) = settings_weak.upgrade() {
                let window = settings.window();
                if let Err(err) = window.show() {
                    error!(error = %err, "failed to show settings window");
                    return;
                }
                // some backends don't schedule an initial paint when showing a window from a menu.
                // request_redraw() ensures the first frame actually gets rendered.
                window.request_redraw();
            }
        });

        self.windows.main.on_quit(|| {
            debug!("quit requested");
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
        let main = MainWindow::new().context("failed to create main window")?;
        main.window().set_size(LogicalSize::new(1000.0, 800.0));
        let settings = SettingsWindow::new().context("failed to create settings window")?;
        settings.window().set_size(LogicalSize::new(800.0, 600.0));

        Ok(Self { main, settings })
    }
}
