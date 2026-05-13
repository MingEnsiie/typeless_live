//! typeless-inject: 文本注入抽象 + 各平台实现。
use async_trait::async_trait;

#[async_trait]
pub trait TextInjector: Send + Sync {
    async fn inject(&self, text: &str) -> anyhow::Result<()>;
    fn name(&self) -> &str;
}

pub mod clipboard;
#[cfg(target_os = "linux")]
pub mod linux;

pub fn default_injector() -> Box<dyn TextInjector> {
    #[cfg(target_os = "linux")]
    { Box::new(linux::LinuxInjector::new()) }
    #[cfg(not(target_os = "linux"))]
    { Box::new(clipboard::ClipboardInjector) }
}
