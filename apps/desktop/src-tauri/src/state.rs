use parking_lot::Mutex;
use std::sync::Arc;
use typeless_core::engine::Engine;
use typeless_llm::{GenOpts, LlmProvider, PostProcessor, PromptMode};
use typeless_storage::{AppPaths, Db, Settings};

pub struct AppState {
    pub paths: AppPaths,
    pub settings: Mutex<Settings>,
    pub db: Arc<Db>,
    pub engine: Arc<Engine>,
}

pub struct HotkeyHolder(pub typeless_hotkey::Hotkey);

pub fn build_provider(settings: &Settings, force_mock: bool) -> Arc<dyn LlmProvider> {
    if force_mock || settings.llm.provider == "mock" {
        return Arc::new(typeless_llm::MockLlm);
    }
    let key = settings.llm.api_key.clone().unwrap_or_default();
    if key.is_empty() {
        return Arc::new(typeless_llm::MockLlm);
    }
    match settings.llm.provider.as_str() {
        "deepseek" => Arc::new(typeless_llm::DeepSeek::new(key, settings.llm.base_url.clone())),
        "mimo" => Arc::new(typeless_llm::MiMo::new(key, settings.llm.base_url.clone())),
        "openai" => {
            let base = settings.llm.base_url.clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".into());
            Arc::new(typeless_llm::OpenAiCompat::new("openai", base, key))
        }
        _ => Arc::new(typeless_llm::MockLlm),
    }
}

pub fn build_asr(settings: &Settings, paths: &AppPaths) -> Arc<dyn typeless_asr::AsrEngine> {
    #[cfg(feature = "whisper")]
    {
        if settings.asr.backend == "whisper" {
            let model_path = settings.asr.model_path.clone()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| paths.models_dir.join(&settings.asr.model));
            if model_path.exists() {
                if let Ok(w) = typeless_asr::WhisperAsr::load(model_path) {
                    return Arc::new(w);
                }
            }
        }
    }
    let _ = (settings, paths);
    Arc::new(typeless_asr::MockAsr)
}

pub fn build_post(settings: &Settings, provider: Arc<dyn LlmProvider>, db: Option<&Db>) -> Arc<PostProcessor> {
    let mut opts = GenOpts::default();
    opts.model = settings.llm.model.clone();
    opts.temperature = settings.llm.temperature;
    opts.max_tokens = settings.llm.max_tokens;
    let mut p = PostProcessor::new(provider, opts);
    p.mode = PromptMode::parse(&settings.prompt_mode);
    if let Some(db) = db {
        if let Ok(list) = db.dict_list() {
            p.dictionary = list.into_iter().map(|e| (e.from_text, e.to_text)).collect();
        }
    }
    p.app_context = typeless_context::AppContext::detect().summary();
    Arc::new(p)
}
