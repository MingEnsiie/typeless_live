use crate::{Delta, GenOpts, LlmProvider, Message};
use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};

/// 测试用 Mock：去除常见口癖、加结束标点。
pub struct MockLlm;

fn refine(text: &str) -> String {
    let mut out = text.to_string();
    for w in ["嗯", "啊", "那个", "就是", "这个", "呃", "[mock asr 录音", "]"] {
        out = out.replace(w, "");
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join("");
    let mut out = out.trim().trim_end_matches(',').to_string();
    if !out.is_empty() && !out.ends_with(['。', '.', '!', '?', '！', '？']) {
        out.push('。');
    }
    out
}

#[async_trait]
impl LlmProvider for MockLlm {
    fn name(&self) -> &str { "mock" }
    async fn complete(&self, msgs: Vec<Message>, _opts: &GenOpts) -> anyhow::Result<String> {
        let user = msgs.last().map(|m| m.content.as_str()).unwrap_or("");
        Ok(refine(user))
    }
    async fn stream(&self, msgs: Vec<Message>, _opts: &GenOpts) -> anyhow::Result<BoxStream<'static, anyhow::Result<Delta>>> {
        let user = msgs.last().map(|m| m.content.clone()).unwrap_or_default();
        let r = refine(&user);
        let chunks: Vec<_> = r.chars().map(|c| Ok(Delta { content: c.to_string() })).collect();
        Ok(stream::iter(chunks).boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn mock_refines() {
        let m = MockLlm;
        let out = m.complete(vec![Message::user("嗯 啊 那个 你好 啊")], &GenOpts::default()).await.unwrap();
        assert_eq!(out, "你好。");
    }
}
