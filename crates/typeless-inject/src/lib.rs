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
#[cfg(target_os = "linux")]
pub mod wayland;
#[cfg(target_os = "windows")]
pub mod windows;

pub fn default_injector() -> Box<dyn TextInjector> {
    #[cfg(target_os = "linux")]
    {
        if wayland::WaylandInjector::detect_session() == "wayland" {
            tracing::info!("wayland session detected; using clipboard injector");
        }
        Box::new(linux::LinuxInjector::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsInjector::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Box::new(clipboard::ClipboardInjector)
    }
}
