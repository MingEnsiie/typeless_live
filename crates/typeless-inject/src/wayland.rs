//! Wayland 文本注入（P2 #26 - skeleton）。
//!
//! 在 Wayland session 中，`enigo`/`xdotool` 不能模拟键盘输入到任意应用。
//! 正确路径有两条：
//!   1. `zwp_virtual_keyboard_v1` 协议（virtual-keyboard-unstable-v1）：
//!      合成器需开启对应 capability（KWin/Hyprland/Sway 默认开启，GNOME 关闭）。
//!   2. `wlr-data-control-unstable-v1` 写剪贴板 + 由用户/IM 触发粘贴。
//!
//! 当前实现：检测会话类型，若为 Wayland 则记录信息并由上层 fallback 到剪贴板注入。
//! 完整实现需要 `wayland-client` + `wayland-protocols-misc`，留作后续工作。

use crate::TextInjector;
use async_trait::async_trait;

pub struct WaylandInjector {
    pub has_virtual_keyboard: bool,
}

impl WaylandInjector {
    pub fn new() -> Self {
        let has_vk = std::env::var("WAYLAND_DISPLAY").is_ok()
            && (std::env::var("XDG_CURRENT_DESKTOP")
                .map(|d| {
                    let d = d.to_lowercase();
                    d.contains("hypr") || d.contains("sway") || d.contains("kde") || d.contains("wlroots")
                })
                .unwrap_or(false));
        Self { has_virtual_keyboard: has_vk }
    }

    pub fn detect_session() -> &'static str {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            "wayland"
        } else if std::env::var("DISPLAY").is_ok() {
            "x11"
        } else {
            "unknown"
        }
    }
}

#[async_trait]
impl TextInjector for WaylandInjector {
    fn name(&self) -> &str {
        "wayland"
    }
    async fn inject(&self, _text: &str) -> anyhow::Result<()> {
        // TODO: 通过 zwp_virtual_keyboard_v1 协议合成按键事件。
        // 当前返回错误，由上层 fallback 到 clipboard injector。
        anyhow::bail!(
            "wayland virtual-keyboard 注入尚未实现 (vk={}); 请使用 ClipboardInjector + 手动 Ctrl+V",
            self.has_virtual_keyboard
        );
    }
}
