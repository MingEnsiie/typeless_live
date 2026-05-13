//! typeless-llm: LLM Provider 抽象 + DeepSeek/MiMo/Mock 实现。
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

pub mod pipeline;
pub mod prompt;
pub mod providers;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "system" | "user" | "assistant"
    pub content: String,
}
impl Message {
    pub fn system(s: impl Into<String>) -> Self { Self { role: "system".into(), content: s.into() } }
    pub fn user(s: impl Into<String>) -> Self { Self { role: "user".into(), content: s.into() } }
    pub fn assistant(s: impl Into<String>) -> Self { Self { role: "assistant".into(), content: s.into() } }
}

#[derive(Debug, Clone)]
pub struct GenOpts {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub stream: bool,
}
impl Default for GenOpts {
    fn default() -> Self {
        Self { model: "deepseek-chat".into(), temperature: 0.3, max_tokens: 1024, stream: false }
    }
}

#[derive(Debug, Clone)]
pub struct Delta { pub content: String }

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn complete(&self, msgs: Vec<Message>, opts: &GenOpts) -> anyhow::Result<String>;
    async fn stream(
        &self,
        msgs: Vec<Message>,
        opts: &GenOpts,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<Delta>>>;
}

pub use pipeline::{PostProcessor, PromptMode};
pub use providers::{deepseek::DeepSeek, mimo::MiMo, mock::MockLlm, openai_compat::OpenAiCompat};
