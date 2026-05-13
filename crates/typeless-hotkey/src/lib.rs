//! 全局快捷键封装。注意：global-hotkey 依赖事件循环，
//! 守护进程模式需调用方在主线程驱动 GlobalHotKeyEvent::receiver()。
use anyhow::Result;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::str::FromStr;
use std::sync::mpsc;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub enum HotkeyEvent {
    Press,
    Release,
}

pub struct Hotkey {
    _mgr: GlobalHotKeyManager,
    _hk: HotKey,
}

impl Hotkey {
    /// 注册一个全局快捷键，返回事件接收器。
    /// 调用方需在主线程定期 `pump()` 以读取事件（或使用 receiver）。
    pub fn register(combo: &str) -> Result<(Self, mpsc::Receiver<HotkeyEvent>)> {
        let mgr = GlobalHotKeyManager::new()
            .map_err(|e| anyhow::anyhow!("hotkey mgr: {e}"))?;
        let hk = HotKey::from_str(combo)
            .map_err(|e| anyhow::anyhow!("invalid hotkey '{combo}': {e}"))?;
        mgr.register(hk).map_err(|e| anyhow::anyhow!("hotkey register: {e}"))?;
        info!(combo=%combo, "hotkey registered");

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let r = GlobalHotKeyEvent::receiver();
            while let Ok(ev) = r.recv() {
                let out = match ev.state {
                    HotKeyState::Pressed => HotkeyEvent::Press,
                    HotKeyState::Released => HotkeyEvent::Release,
                };
                if tx.send(out).is_err() { break; }
            }
            warn!("hotkey channel closed");
        });

        Ok((Self { _mgr: mgr, _hk: hk }, rx))
    }
}
