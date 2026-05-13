use crate::{providers::openai_compat::OpenAiCompat, Delta, GenOpts, LlmProvider, Message};
use async_trait::async_trait;
use futures::stream::BoxStream;

pub struct DeepSeek { inner: OpenAiCompat }

impl DeepSeek {
    pub fn new(api_key: impl Into<String>, base_url: Option<String>) -> Self {
        let url = base_url.unwrap_or_else(|| "https://api.deepseek.com/v1".into());
        Self { inner: OpenAiCompat::new("deepseek", url, api_key) }
    }
}

#[async_trait]
impl LlmProvider for DeepSeek {
    fn name(&self) -> &str { "deepseek" }
    async fn complete(&self, msgs: Vec<Message>, opts: &GenOpts) -> anyhow::Result<String> {
        self.inner.complete(msgs, opts).await
    }
    async fn stream(&self, msgs: Vec<Message>, opts: &GenOpts) -> anyhow::Result<BoxStream<'static, anyhow::Result<Delta>>> {
        self.inner.stream(msgs, opts).await
    }
}
