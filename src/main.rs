use std::{ops::Deref, sync::Arc};

use anyhow::{Context, Result};
use slint::LogicalSize;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::{api::typing::Session, settings::Settings};

mod api;
mod secrets;
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

    info!("loading settings");
    let settings = Arc::new(RwLock::new(Settings::load_or_default()));

    info!("initialize keyring");
    secrets::init_keyring();

    info!("load typing.com api");
    let typing_session = Session::login(&*settings.read().await)
        .await
        .map_err(|error| {
            warn!(%error, "failed to create typing.com session");
            error
        })
        .ok();

    info!("loading ui");
    let windows = AppWindows::new().unwrap_or_else(|error| {
        error!(%error, "failed to load UI windows");
        std::process::exit(1);
    });
    slint::set_xdg_app_id(APP_NAME)
        .unwrap_or_else(|error| error!(%error, "failed to register XDG app id"));
    debug!("implement ui callbacks");
    windows.impl_callbacks(&AppContext {
        settings: Arc::clone(&settings),
        typing_session: typing_session.clone(),
    });

    debug!("show main window");
    windows.main.run().unwrap_or_else(|error| {
        error!(%error, "slint platform crashed");
        std::process::exit(1);
    });

    info!("exiting app");
}

fn quit() {
    debug!("unset default keyring store");
    keyring_core::unset_default_store();

    debug!("quit slint event loop");
    slint::quit_event_loop()
        .unwrap_or_else(|error| error!(%error, "failed to quit slint event loop"));
}

struct AppContext {
    settings: Arc<RwLock<Settings>>,
    typing_session: Option<Session>,
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

    fn impl_callbacks(&self, ctx: &AppContext) {
        self.main.window().on_close_requested(move || {
            debug!("close requested");
            quit();

            slint::CloseRequestResponse::HideWindow
        });

        let settings = Arc::clone(&ctx.settings);
        self.settings.window().on_close_requested(move || {
            debug!("close settings window");
            info!("saving settings");
            let settings = Arc::clone(&settings);
            tokio::spawn(async move {
                settings
                    .read()
                    .await
                    .save()
                    .unwrap_or_else(|error| error!(%error, "failed to save settings"));
            });

            slint::CloseRequestResponse::HideWindow
        });

        self.main
            .on_check_for_updates(|| warn!("check for updates not implemented yet"));

        let settings_weak = self.settings.as_weak();
        self.main.on_open_settings(move || {
            debug!("opening settings window");
            if let Some(settings_window) = settings_weak.upgrade() {
                let window = settings_window.window();
                if let Err(error) = window.show() {
                    error!(%error, "failed to show settings window");
                    return;
                }
                // some backends don't schedule an initial paint when showing a window from a menu.
                // request_redraw() ensures the first frame actually gets rendered.
                window.request_redraw();
            }
        });

        self.main.on_quit(move || {
            debug!("quit requested");
            quit();
        });

        self.main
            .on_fetch_students(|| warn!("fetch students not implemented yet"));
    }
}
