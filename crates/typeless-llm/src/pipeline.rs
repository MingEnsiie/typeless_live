use crate::{prompt, GenOpts, LlmProvider, Message};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub enum PromptMode {
    Default,
    Email,
    Code,
    TranslateEn,
    Formal,
}
impl PromptMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PromptMode::Default => "default",
            PromptMode::Email => "email",
            PromptMode::Code => "code",
            PromptMode::TranslateEn => "translate_en",
            PromptMode::Formal => "formal",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "email" => Self::Email,
            "code" => Self::Code,
            "translate_en" | "translate-en" | "en" => Self::TranslateEn,
            "formal" => Self::Formal,
            _ => Self::Default,
        }
    }
}

pub struct PostProcessor {
    pub provider: Arc<dyn LlmProvider>,
    pub opts: GenOpts,
    pub mode: PromptMode,
    /// 词典：按顺序在送 LLM 前做正则/字面替换。
    pub dictionary: Vec<(String, String)>,
    /// 当前应用上下文（用于注入 prompt）
    pub app_context: Option<String>,
    /// 用户语言（zh/en/ja/ko/auto），用于选择默认 system prompt
    pub language: String,
}

impl PostProcessor {
    pub fn new(provider: Arc<dyn LlmProvider>, opts: GenOpts) -> Self {
        Self {
            provider, opts,
            mode: PromptMode::Default,
            dictionary: Vec::new(),
            app_context: None,
            language: "zh".into(),
        }
    }

    fn apply_dict(&self, mut text: String) -> String {
        for (from, to) in &self.dictionary {
            if !from.is_empty() {
                text = text.replace(from.as_str(), to);
            }
        }
        text
    }

    fn build_messages(&self, raw: &str) -> Vec<Message> {
        let pre = self.apply_dict(raw.to_string());
        let mut sys = String::from(prompt::system_for_lang_mode(&self.language, self.mode.as_str()));
        if !self.dictionary.is_empty() {
            sys.push_str("\n\n参考术语映射（已预处理但请保持一致）：\n");
            for (f, t) in &self.dictionary {
                sys.push_str(&format!("- {f} → {t}\n"));
            }
        }
        if let Some(ctx) = &self.app_context {
            sys.push_str(&format!("\n当前用户应用上下文：{ctx}\n"));
        }
        vec![Message::system(sys), Message::user(pre)]
    }

    pub async fn refine(&self, raw: &str) -> anyhow::Result<String> {
        if raw.trim().is_empty() {
            return Ok(String::new());
        }
        let msgs = self.build_messages(raw);
        let out = self.provider.complete(msgs, &self.opts).await?;
        Ok(out.trim().to_string())
    }

    pub async fn refine_stream(
        &self,
        raw: &str,
    ) -> anyhow::Result<futures::stream::BoxStream<'static, anyhow::Result<crate::Delta>>> {
        let msgs = self.build_messages(raw);
        let mut o = self.opts.clone();
        o.stream = true;
        self.provider.stream(msgs, &o).await
    }
}
