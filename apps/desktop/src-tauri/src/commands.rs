use crate::state::AppState;
use std::path::PathBuf;
use tauri::State;
use typeless_storage::{HistoryRecord, Settings};

#[tauri::command]
pub async fn toggle_recording(state: State<'_, AppState>) -> Result<(), String> {
    let engine = state.engine.clone();
    if engine.is_recording() {
        engine.stop_and_process().await.map_err(|e| e.to_string())?;
    } else {
        engine.start_recording().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().clone()
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    settings.save(&state.paths.config_file).map_err(|e| e.to_string())?;
    *state.settings.lock() = settings;
    Ok(())
}

#[tauri::command]
pub fn list_history(state: State<AppState>, limit: usize) -> Result<Vec<HistoryRecord>, String> {
    state.db.list_history(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn dict_list(state: State<AppState>) -> Result<Vec<typeless_storage::DictEntry>, String> {
    state.db.dict_list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn dict_add(state: State<AppState>, from: String, to: String, note: Option<String>) -> Result<(), String> {
    state.db.dict_upsert(&from, &to, note.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn dict_remove(state: State<AppState>, id: i64) -> Result<(), String> {
    state.db.dict_delete(id).map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct ModelInfo {
    pub kind: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
}

#[tauri::command]
pub fn list_models(state: State<AppState>) -> Result<Vec<ModelInfo>, String> {
    let dir = &state.paths.models_dir;
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_file() {
                let size = ent.metadata().map(|m| m.len()).unwrap_or(0);
                out.push(ModelInfo {
                    kind: if p.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("ggml")).unwrap_or(false) { "whisper".into() } else { "llm".into() },
                    name: p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
                    path: p.to_string_lossy().into(),
                    size_bytes: size,
                });
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn download_model(state: State<'_, AppState>, kind: String, name: String, url: String) -> Result<String, String> {
    let target: PathBuf = state.paths.models_dir.join(&name);
    let _ = kind;
    let resp = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    std::fs::write(&target, &bytes).map_err(|e| e.to_string())?;
    Ok(target.to_string_lossy().into())
}
