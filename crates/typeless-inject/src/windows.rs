#![cfg(target_os = "windows")]
//! Windows 文本注入：剪贴板 + Ctrl+V 模拟。
use crate::TextInjector;
use async_trait::async_trait;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use tracing::warn;

pub struct WindowsInjector;

impl WindowsInjector {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl TextInjector for WindowsInjector {
    fn name(&self) -> &str { "windows-clipboard-paste" }

    async fn inject(&self, text: &str) -> anyhow::Result<()> {
        let text = text.to_string();
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut cb = arboard::Clipboard::new()?;
            let prev = cb.get_text().ok();
            cb.set_text(text.clone())?;
            std::thread::sleep(std::time::Duration::from_millis(60));

            let mut en = Enigo::new(&Settings::default())
                .map_err(|e| anyhow::anyhow!("enigo init: {e:?}"))?;
            en.key(Key::Control, Direction::Press)
                .map_err(|e| anyhow::anyhow!("ctrl press: {e:?}"))?;
            en.key(Key::Unicode('v'), Direction::Click)
                .map_err(|e| anyhow::anyhow!("v click: {e:?}"))?;
            en.key(Key::Control, Direction::Release)
                .map_err(|e| anyhow::anyhow!("ctrl release: {e:?}"))?;

            std::thread::sleep(std::time::Duration::from_millis(150));
            if let Some(p) = prev {
                if let Err(e) = cb.set_text(p) {
                    warn!(error=%e, "restore clipboard failed");
                }
            }
            Ok(())
        }).await??;
        Ok(())
    }
}
