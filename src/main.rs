use std::rc::Rc;

use anyhow::{Context, Result};
use slint::LogicalSize;
use tracing::{debug, error, warn};
use tracing_subscriber::EnvFilter;

use crate::settings::Settings;

mod api;
mod settings;

slint::include_modules!();

const APP_NAME: &str = "keystroke";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(format!("{APP_NAME}=info,slint=warn"))),
        )
        .init();

    debug!("starting app");
    debug!("loading settings");
    let ctx = Rc::new(AppContext::new().unwrap_or_else(|err| {
        error!(error = %err, "failed to create app context");
        std::process::exit(1);
    }));
    slint::set_xdg_app_id(APP_NAME)
        .unwrap_or_else(|err| error!(error = %err, "failed to register XDG app ID"));
    debug!("implement UI callbacks");
    ctx.impl_callbacks();

    debug!("load typing.com api");
    let _typing_session = api::typing::login(&ctx).await.unwrap_or_else(|err| {
        error!(error = %err, "login to typing.com failed");
        api::typing::Session::default()
    });

    debug!("show main window");
    ctx.windows.main.run().unwrap_or_else(|err| {
        error!(error = %err, "slint platform crashed");
        std::process::exit(1);
    });

    debug!("exiting app");
}

fn save_settings_and_exit(ctx: &AppContext) {
    ctx.settings
        .save()
        .unwrap_or_else(|err| error!(error = %err, "failed to save settings"));

    slint::quit_event_loop()
        .unwrap_or_else(|err| error!(error = %err, "error encountered while quitting app"));
}

struct AppContext {
    windows: AppWindows,
    settings: Settings,
}

impl AppContext {
    fn new() -> Result<Self> {
        Ok(Self {
            windows: AppWindows::new()?,
            settings: Settings::load_or_default(),
        })
    }

    fn impl_callbacks(self: &Rc<Self>) {
        let close_ctx = Rc::downgrade(self);
        self.windows.main.window().on_close_requested(move || {
            debug!("close requested");
            if let Some(ctx) = close_ctx.upgrade() {
                save_settings_and_exit(&ctx);
            }
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

        let quit_ctx = Rc::downgrade(self);
        self.windows.main.on_quit(move || {
            debug!("quit requested");
            if let Some(ctx) = quit_ctx.upgrade() {
                save_settings_and_exit(&ctx);
            }
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
