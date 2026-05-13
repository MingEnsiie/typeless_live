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

pub fn default_injector() -> Box<dyn TextInjector> {
    #[cfg(target_os = "linux")]
    {
        // Wayland 优先：尝试 virtual-keyboard，未实现则上层会回退 clipboard。
        if wayland::WaylandInjector::detect_session() == "wayland" {
            tracing::info!("wayland session detected; using clipboard injector (virtual-keyboard 未实现)");
        }
        Box::new(linux::LinuxInjector::new())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Box::new(clipboard::ClipboardInjector)
    }
}
