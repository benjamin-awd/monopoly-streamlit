use log::{error, info, warn};
use std::env;

use killport::cli::Mode;
use killport::killport::{Killport, KillportOperations};
use killport::signal::KillportSignal;
use reqwest::Client;
use std::time::Duration;
use tauri::{Manager, WindowEvent};
use tauri_plugin_shell::ShellExt;
use tokio::time::sleep;

pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let splash_window = app.get_webview_window("splashscreen").unwrap();
            let main_window = app.get_webview_window("main").unwrap();
            let sidecar = app.shell().sidecar("statementsensei").unwrap();

            tauri::async_runtime::spawn(async move {
                let (_rx, _child) = sidecar.spawn().expect("Failed to spawn sidecar");
                let client = Client::new();

                loop {
                    match client.get("http://localhost:8501").send().await {
                        Ok(response) if response.status().is_success() => {
                            info!("Streamlit server loaded");
                            sleep(Duration::from_millis(500)).await;
                            break;
                        }
                        _ => {
                            warn!("Streamlit server not available, retrying...");
                            sleep(Duration::from_millis(500)).await;
                        }
                    }
                }
                main_window
                    .eval("window.location.replace('http://localhost:8501');")
                    .expect("Failed to load the URL in the main window");

                sleep(Duration::from_millis(250)).await;
                splash_window.hide().unwrap();
                main_window.show().unwrap();
            });

            Ok(())
        })
        .on_window_event(|window, event| match event {
            #[allow(unused_variables)]
            WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "splashscreen" {
                    info!("Close requested - exiting app");
                    kill_statementsensei(8501);
                    window.app_handle().exit(0);
                }
            }
            WindowEvent::Destroyed => {
                if window.label() == "main" {
                    info!("Window destroyed - exiting app");
                    kill_statementsensei(8501);
                    window.app_handle().exit(0);
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn kill_statementsensei(port: u16) {
    let killport = Killport;
    let mode = Mode::Auto;

    let target_killables = match killport.find_target_killables(port, mode) {
        Ok(killables) => killables,
        Err(err) => {
            error!("Error finding killables: {}", err);
            return;
        }
    };

    for killable in target_killables {
        if killable.get_name().contains("statementsensei") {
            let signal: KillportSignal = "SIGKILL".parse().unwrap();

            if let Err(err) = killable.kill(signal) {
                error!("Error killing {}: {}", killable.get_name(), err);
            } else {
                info!("Killed {}", killable.get_name());
            }
        }
    }
}
