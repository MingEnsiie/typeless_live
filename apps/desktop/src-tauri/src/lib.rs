//! Tauri 桌面应用：暴露 typeless-core 命令、监听状态事件并广播给前端。
mod commands;
mod state;

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tracing_subscriber::EnvFilter;
use typeless_core::engine::{Engine, EngineConfig};
use typeless_storage::{AppPaths, Db, Settings};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let paths = AppPaths::discover()?;
            let settings = Settings::load_or_create(&paths.config_file)?;
            let db = Arc::new(Db::open(&paths.db_file)?);

            let provider = state::build_provider(&settings, false);
            let asr = state::build_asr(&settings, &paths);
            let post = state::build_post(&settings, provider, Some(&db));
            let injector: Arc<dyn typeless_inject::TextInjector> =
                Arc::from(typeless_inject::default_injector());
            let cfg = EngineConfig {
                mode: typeless_audio::CaptureMode::Toggle,
                language: if settings.asr.language == "auto" { None } else { Some(settings.asr.language.clone()) },
                translate: settings.asr.translate,
                save_history: !settings.privacy.no_history,
                auto_inject: true,
            };
            let engine = Arc::new(Engine::new(asr, post, injector, Some(db.clone()), cfg));

            app.manage(state::AppState {
                paths,
                settings: parking_lot::Mutex::new(settings.clone()),
                db,
                engine: engine.clone(),
            });

            // 转发引擎事件 → 前端
            let app_handle = app.handle().clone();
            let mut rx = engine.subscribe();
            tauri::async_runtime::spawn(async move {
                use typeless_core::state::StatusEvent;
                while let Ok(ev) = rx.recv().await {
                    match ev {
                        StatusEvent::Status { status } => {
                            let _ = app_handle.emit("status", serde_json::json!({"status": status}));
                        }
                        StatusEvent::PartialText { text } => {
                            let _ = app_handle.emit("partial", serde_json::json!({"text": text}));
                        }
                        StatusEvent::FinalText { text } => {
                            let _ = app_handle.emit("final", serde_json::json!({"text": text}));
                        }
                        StatusEvent::Error { message } => {
                            let _ = app_handle.emit("error", serde_json::json!({"message": message}));
                        }
                        StatusEvent::Info { message } => {
                            let _ = app_handle.emit("info", serde_json::json!({"message": message}));
                        }
                    }
                }
            });

            // 注册全局热键
            let combo = settings.hotkey.trigger.clone();
            match typeless_hotkey::Hotkey::register(&combo) {
                Ok((hk, rx_hk)) => {
                    app.manage(state::HotkeyHolder(hk));
                    let engine2 = engine.clone();
                    std::thread::spawn(move || {
                        while let Ok(ev) = rx_hk.recv() {
                            if let typeless_hotkey::HotkeyEvent::Press = ev {
                                let e = engine2.clone();
                                tauri::async_runtime::spawn(async move {
                                    if e.is_recording() {
                                        let _ = e.stop_and_process().await;
                                    } else {
                                        let _ = e.start_recording();
                                    }
                                });
                            }
                        }
                    });
                }
                Err(e) => tracing::warn!(error=%e, "hotkey registration failed"),
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::toggle_recording,
            commands::get_settings,
            commands::save_settings,
            commands::list_history,
            commands::dict_list,
            commands::dict_add,
            commands::dict_remove,
            commands::list_models,
            commands::download_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
