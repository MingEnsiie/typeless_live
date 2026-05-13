use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub asr: AsrSettings,
    pub llm: LlmSettings,
    pub hotkey: HotkeySettings,
    pub ui: UiSettings,
    pub privacy: PrivacySettings,
    pub prompt_mode: String,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            asr: Default::default(),
            llm: Default::default(),
            hotkey: Default::default(),
            ui: Default::default(),
            privacy: Default::default(),
            prompt_mode: "default".into(),
            language: "auto".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AsrSettings {
    pub backend: String,
    pub model: String,
    pub model_path: Option<String>,
    pub language: String,
    pub translate: bool,
}
impl Default for AsrSettings {
    fn default() -> Self {
        Self {
            backend: "whisper".into(),
            model: "ggml-base.bin".into(),
            model_path: None,
            language: "auto".into(),
            translate: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmSettings {
    pub provider: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
}
impl Default for LlmSettings {
    fn default() -> Self {
        Self {
            provider: "deepseek".into(),
            model: "deepseek-chat".into(),
            base_url: None,
            api_key: None,
            temperature: 0.3,
            max_tokens: 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeySettings {
    pub trigger: String,
    pub mode: String,
}
impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            trigger: "Ctrl+Alt+Space".into(),
            mode: "toggle".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub show_floating: bool,
    pub theme: String,
}
impl Default for UiSettings {
    fn default() -> Self { Self { show_floating: true, theme: "auto".into() } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacySettings {
    pub local_only: bool,
    pub no_history: bool,
    pub keep_audio: bool,
}
impl Default for PrivacySettings {
    fn default() -> Self {
        Self { local_only: false, no_history: false, keep_audio: false }
    }
}

impl Settings {
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            let txt = std::fs::read_to_string(path)?;
            Ok(toml::from_str(&txt).unwrap_or_default())
        } else {
            let s = Self::default();
            s.save(path)?;
            Ok(s)
        }
    }
    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        if let Some(p) = path.as_ref().parent() {
            std::fs::create_dir_all(p).ok();
        }
        let txt = toml::to_string_pretty(self)?;
        std::fs::write(path, txt)?;
        Ok(())
    }

    pub fn set_dotted(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        // 简易 dotted-key 设置（仅支持已知路径）
        let mut parts = key.splitn(2, '.');
        let head = parts.next().unwrap_or("");
        let tail = parts.next().unwrap_or("");
        match (head, tail) {
            ("asr", "backend") => self.asr.backend = value.into(),
            ("asr", "model") => self.asr.model = value.into(),
            ("asr", "language") => self.asr.language = value.into(),
            ("asr", "translate") => self.asr.translate = value.parse().unwrap_or(false),
            ("llm", "provider") => self.llm.provider = value.into(),
            ("llm", "model") => self.llm.model = value.into(),
            ("llm", "base_url") => self.llm.base_url = Some(value.into()),
            ("llm", "api_key") => self.llm.api_key = Some(value.into()),
            ("llm", "temperature") => self.llm.temperature = value.parse().unwrap_or(0.3),
            ("hotkey", "trigger") => self.hotkey.trigger = value.into(),
            ("hotkey", "mode") => self.hotkey.mode = value.into(),
            ("ui", "show_floating") => self.ui.show_floating = value.parse().unwrap_or(true),
            ("ui", "theme") => self.ui.theme = value.into(),
            ("privacy", "local_only") => self.privacy.local_only = value.parse().unwrap_or(false),
            ("privacy", "no_history") => self.privacy.no_history = value.parse().unwrap_or(false),
            ("", "prompt_mode") | ("prompt_mode", "") => self.prompt_mode = value.into(),
            ("", "language") | ("language", "") => self.language = value.into(),
            _ => return Err(anyhow::anyhow!("unknown key: {key}")),
        }
        Ok(())
    }
}
