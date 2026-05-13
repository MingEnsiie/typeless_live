//! 小米 MiMo Token Plan provider。
//! 默认按 OpenAI 兼容协议接入；用户可在 base_url 指定官方 endpoint。
use crate::{providers::openai_compat::OpenAiCompat, Delta, GenOpts, LlmProvider, Message};
use async_trait::async_trait;
use futures::stream::BoxStream;

pub struct MiMo { inner: OpenAiCompat }

impl MiMo {
    pub fn new(api_key: impl Into<String>, base_url: Option<String>) -> Self {
        // Placeholder default; 用户需在配置中提供官方 base_url。
        let url = base_url.unwrap_or_else(|| "https://api.mimo.xiaomi.com/v1".into());
        Self { inner: OpenAiCompat::new("mimo", url, api_key) }
    }
}

#[async_trait]
impl LlmProvider for MiMo {
    fn name(&self) -> &str { "mimo" }
    async fn complete(&self, msgs: Vec<Message>, opts: &GenOpts) -> anyhow::Result<String> {
        self.inner.complete(msgs, opts).await
    }
    async fn stream(&self, msgs: Vec<Message>, opts: &GenOpts) -> anyhow::Result<BoxStream<'static, anyhow::Result<Delta>>> {
        self.inner.stream(msgs, opts).await
    }
}
