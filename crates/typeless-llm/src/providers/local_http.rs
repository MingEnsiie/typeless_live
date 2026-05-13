//! 本地 LLM provider：通过 HTTP 调用本地推理服务（llama.cpp server / Ollama / vLLM）。
//! 这种实现避免了 llama-cpp-rs 的复杂 C++ 依赖，可即装即用。
//!
//! 使用方式：
//! - llama.cpp server: `./server -m model.gguf --port 8080` → base_url=http://localhost:8080/v1
//! - Ollama: 默认 base_url=http://localhost:11434/v1
use crate::{providers::openai_compat::OpenAiCompat, Delta, GenOpts, LlmProvider, Message};
use async_trait::async_trait;
use futures::stream::BoxStream;

pub struct LocalHttp {
    inner: OpenAiCompat,
}

impl LocalHttp {
    pub fn new(base_url: Option<String>) -> Self {
        let url = base_url.unwrap_or_else(|| "http://localhost:8080/v1".into());
        Self {
            inner: OpenAiCompat::new("local", url, "no-key-needed"),
        }
    }
}

#[async_trait]
impl LlmProvider for LocalHttp {
    fn name(&self) -> &str {
        "local"
    }
    async fn complete(&self, msgs: Vec<Message>, opts: &GenOpts) -> anyhow::Result<String> {
        self.inner.complete(msgs, opts).await
    }
    async fn stream(
        &self,
        msgs: Vec<Message>,
        opts: &GenOpts,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<Delta>>> {
        self.inner.stream(msgs, opts).await
    }
}
