//! OpenAI 兼容 provider：DeepSeek/MiMo/任意 OAI 兼容服务的基础实现。
use crate::{Delta, GenOpts, LlmProvider, Message};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct OpenAiCompat {
    pub provider_name: String,
    pub base_url: String,
    pub api_key: String,
    pub client: Client,
}

impl OpenAiCompat {
    pub fn new(provider_name: impl Into<String>, base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("http client");
        Self {
            provider_name: provider_name.into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            client,
        }
    }
    fn endpoint(&self) -> String {
        let b = self.base_url.trim_end_matches('/');
        format!("{b}/chat/completions")
    }
}

#[derive(Serialize)]
struct ChatReq<'a> {
    model: &'a str,
    messages: &'a [Message],
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: Option<MsgContent>,
}
#[derive(Deserialize)]
struct MsgContent { content: Option<String> }
#[derive(Deserialize)]
struct ChatResp { choices: Vec<ChatChoice> }

#[derive(Deserialize)]
struct StreamChoice { delta: Option<MsgContent> }
#[derive(Deserialize)]
struct StreamChunk { choices: Vec<StreamChoice> }

#[async_trait]
impl LlmProvider for OpenAiCompat {
    fn name(&self) -> &str { &self.provider_name }

    async fn complete(&self, msgs: Vec<Message>, opts: &GenOpts) -> Result<String> {
        let body = ChatReq {
            model: &opts.model,
            messages: &msgs,
            temperature: opts.temperature,
            max_tokens: opts.max_tokens,
            stream: false,
        };
        let resp = self.client.post(self.endpoint())
            .bearer_auth(&self.api_key)
            .json(&body).send().await?;
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(anyhow!("llm http {s}: {t}"));
        }
        let parsed: ChatResp = resp.json().await?;
        let txt = parsed.choices.into_iter()
            .next()
            .and_then(|c| c.message)
            .and_then(|m| m.content)
            .unwrap_or_default();
        Ok(txt)
    }

    async fn stream(
        &self,
        msgs: Vec<Message>,
        opts: &GenOpts,
    ) -> Result<BoxStream<'static, Result<Delta>>> {
        let body = ChatReq {
            model: &opts.model,
            messages: &msgs,
            temperature: opts.temperature,
            max_tokens: opts.max_tokens,
            stream: true,
        };
        let resp = self.client.post(self.endpoint())
            .bearer_auth(&self.api_key)
            .json(&body).send().await?;
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(anyhow!("llm http {s}: {t}"));
        }
        let stream = resp.bytes_stream().eventsource()
            .filter_map(|ev| async move {
                match ev {
                    Ok(e) => {
                        if e.data == "[DONE]" { return None; }
                        match serde_json::from_str::<StreamChunk>(&e.data) {
                            Ok(c) => {
                                let d = c.choices.into_iter()
                                    .next()
                                    .and_then(|c| c.delta)
                                    .and_then(|m| m.content)
                                    .unwrap_or_default();
                                if d.is_empty() { None } else { Some(Ok(Delta { content: d })) }
                            }
                            Err(_) => None,
                        }
                    }
                    Err(e) => Some(Err(anyhow!("sse error: {e}"))),
                }
            });
        Ok(Box::pin(stream))
    }
}
