#![cfg(target_os = "linux")]
//! Linux 文本注入：剪贴板 + Ctrl+V 粘贴模拟。
//!
//! Unicode 字符直接 send key 在 X11/Wayland 各 IME 下兼容性差，
//! 因此采用：保存当前剪贴板 → 写入新文本 → 模拟 Ctrl+V → 还原剪贴板。
use crate::TextInjector;
use async_trait::async_trait;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use tracing::warn;

pub struct LinuxInjector;

impl LinuxInjector {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl TextInjector for LinuxInjector {
    fn name(&self) -> &str { "linux-clipboard-paste" }

    async fn inject(&self, text: &str) -> anyhow::Result<()> {
        let text = text.to_string();
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut cb = arboard::Clipboard::new()?;
            let prev = cb.get_text().ok();
            cb.set_text(text.clone())?;
            // 短暂等待剪贴板生效
            std::thread::sleep(std::time::Duration::from_millis(50));

            let mut en = Enigo::new(&Settings::default())
                .map_err(|e| anyhow::anyhow!("enigo init: {e:?}"))?;
            en.key(Key::Control, Direction::Press)
                .map_err(|e| anyhow::anyhow!("ctrl press: {e:?}"))?;
            en.key(Key::Unicode('v'), Direction::Click)
                .map_err(|e| anyhow::anyhow!("v click: {e:?}"))?;
            en.key(Key::Control, Direction::Release)
                .map_err(|e| anyhow::anyhow!("ctrl release: {e:?}"))?;

            // 还原剪贴板
            std::thread::sleep(std::time::Duration::from_millis(120));
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
