use crate::TextInjector;
use async_trait::async_trait;

pub struct ClipboardInjector;

#[async_trait]
impl TextInjector for ClipboardInjector {
    fn name(&self) -> &str { "clipboard" }
    async fn inject(&self, text: &str) -> anyhow::Result<()> {
        let text = text.to_string();
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut cb = arboard::Clipboard::new()?;
            cb.set_text(text)?;
            Ok(())
        }).await??;
        Ok(())
    }
}
