use crate::{AsrEngine, AsrOptions, AsrResult};
use async_trait::async_trait;

/// 测试用 Mock ASR：将 pcm 长度映射为占位文字。
pub struct MockAsr;

#[async_trait]
impl AsrEngine for MockAsr {
    async fn transcribe(&self, pcm: &[i16], _opts: &AsrOptions) -> anyhow::Result<AsrResult> {
        let secs = pcm.len() as f64 / 16000.0;
        let text = if secs < 0.2 {
            String::new()
        } else {
            format!("[mock asr 录音 {:.2}s 嗯 这是一段 测试 文本 啊 那个 你好]", secs)
        };
        Ok(AsrResult {
            text, language: Some("zh".into()),
            duration_ms: (secs * 1000.0) as u64,
        })
    }
    fn name(&self) -> &str { "mock" }
}
