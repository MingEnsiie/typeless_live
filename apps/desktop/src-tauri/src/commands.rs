use crate::state::{AppState, build_provider};
use std::path::PathBuf;
use tauri::{Emitter, State, Window};
use typeless_llm::{GenOpts, LlmProvider, Message};
use typeless_storage::{models as model_registry, HistoryRecord, Settings};

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

// ── 模型 ────────────────────────────────────────────────
#[derive(serde::Serialize)]
pub struct ModelInfo {
    pub kind: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub downloaded: bool,
}

#[tauri::command]
pub fn list_models(state: State<AppState>) -> Result<Vec<ModelInfo>, String> {
    let dir = &state.paths.models_dir;
    let registry = model_registry::registry();
    let mut out: Vec<ModelInfo> = registry.iter().map(|m| {
        let p = dir.join(&m.name);
        let downloaded = p.exists();
        ModelInfo {
            kind: m.kind.clone(),
            name: m.name.clone(),
            path: p.to_string_lossy().into(),
            size_bytes: if downloaded { std::fs::metadata(&p).map(|x| x.len()).unwrap_or(0) }
                        else { m.size_mb as u64 * 1_048_576 },
            downloaded,
        }
    }).collect();
    // 也加入目录里未在注册表中的文件
    if let Ok(rd) = std::fs::read_dir(dir) {
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_file() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                if !out.iter().any(|m| m.name == name) {
                    out.push(ModelInfo {
                        kind: if name.starts_with("ggml") { "whisper" } else { "llm" }.into(),
                        name,
                        path: p.to_string_lossy().into(),
                        size_bytes: ent.metadata().map(|m| m.len()).unwrap_or(0),
                        downloaded: true,
                    });
                }
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn download_model(
    state: State<'_, AppState>,
    window: Window,
    name: String,
) -> Result<String, String> {
    let desc = model_registry::find(&name)
        .ok_or_else(|| format!("未知模型: {name}"))?;
    let target: PathBuf = state.paths.models_dir.join(&desc.name);
    if target.exists() {
        return Ok(target.to_string_lossy().into());
    }
    std::fs::create_dir_all(&state.paths.models_dir).map_err(|e| e.to_string())?;

    let resp = reqwest::get(&desc.url).await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    let mut stream = resp.bytes_stream();
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::File::create(&target).await.map_err(|e| e.to_string())?;
    let mut downloaded = 0u64;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        let pct = if total > 0 { downloaded * 100 / total } else { 0 };
        let _ = window.emit("download_progress", serde_json::json!({
            "name": name, "downloaded": downloaded, "total": total, "pct": pct
        }));
    }
    file.flush().await.map_err(|e| e.to_string())?;
    Ok(target.to_string_lossy().into())
}

// ── LLM 连通性测试 ───────────────────────────────────────
#[derive(serde::Serialize)]
pub struct PingResult { pub ok: bool, pub ms: u64, pub reply: String }

#[tauri::command]
pub async fn ping_llm(state: State<'_, AppState>) -> Result<PingResult, String> {
    let settings = state.settings.lock().clone();
    let provider = build_provider(&settings, false);
    let mut opts = GenOpts::default();
    opts.model = settings.llm.model.clone();
    opts.max_tokens = 16;
    let msgs = vec![
        Message::system("Reply with the single word: pong"),
        Message::user("ping"),
    ];
    let t0 = std::time::Instant::now();
    match provider.complete(msgs, &opts).await {
        Ok(reply) => Ok(PingResult { ok: true, ms: t0.elapsed().as_millis() as u64, reply }),
        Err(e)    => Ok(PingResult { ok: false, ms: t0.elapsed().as_millis() as u64, reply: e.to_string() }),
    }
}

// ── Provider 预设列表 ────────────────────────────────────
#[derive(serde::Serialize, Clone)]
pub struct ProviderPreset {
    pub id: String, pub name: String, pub base_url: String, pub default_model: String,
}

#[tauri::command]
pub fn get_providers() -> Vec<ProviderPreset> {
    vec![
        ProviderPreset { id: "mimo".into(),     name: "小米 MiMo".into(),
            base_url: "https://token-plan-cn.xiaomimimo.com/v1".into(),
            default_model: "mimo-v2.5-pro".into() },
        ProviderPreset { id: "deepseek".into(), name: "DeepSeek".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            default_model: "deepseek-chat".into() },
        ProviderPreset { id: "openai".into(),   name: "OpenAI".into(),
            base_url: "https://api.openai.com/v1".into(),
            default_model: "gpt-4o-mini".into() },
        ProviderPreset { id: "local".into(),    name: "本地 (llama.cpp/Ollama)".into(),
            base_url: "http://localhost:11434/v1".into(),
            default_model: "llama3".into() },
        ProviderPreset { id: "mock".into(),     name: "Mock（无需 key）".into(),
            base_url: String::new(), default_model: "mock".into() },
    ]
}
