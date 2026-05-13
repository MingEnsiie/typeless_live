//! typeless-asr: ASR Engine 抽象 + Whisper 实现 + Mock。
use async_trait::async_trait;

pub mod mock;
pub mod streaming;
#[cfg(feature = "whisper")]
pub mod whisper;

#[derive(Debug, Clone)]
pub struct AsrOptions {
    pub language: Option<String>,
    pub translate: bool,
}
impl Default for AsrOptions {
    fn default() -> Self { Self { language: None, translate: false } }
}

#[derive(Debug, Clone)]
pub struct AsrResult {
    pub text: String,
    pub language: Option<String>,
    pub duration_ms: u64,
}

#[async_trait]
pub trait AsrEngine: Send + Sync {
    /// 输入 16kHz / mono / i16 PCM
    async fn transcribe(&self, pcm: &[i16], opts: &AsrOptions) -> anyhow::Result<AsrResult>;
    fn name(&self) -> &str;
}

pub use mock::MockAsr;
#[cfg(feature = "whisper")]
pub use whisper::WhisperAsr;
