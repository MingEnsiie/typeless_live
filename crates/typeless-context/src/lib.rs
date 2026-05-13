//! 检测当前活动应用 / 窗口标题，用于 prompt 注入。
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppContext {
    pub app: Option<String>,
    pub window_title: Option<String>,
    pub display_server: Option<String>, // x11 | wayland
}

impl AppContext {
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        return linux::detect();
        #[cfg(target_os = "windows")]
        return windows::detect();
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        return Self::default();
    }

    pub fn summary(&self) -> Option<String> {
        match (&self.app, &self.window_title) {
            (Some(a), Some(t)) => Some(format!("应用: {a} / 窗口: {t}")),
            (Some(a), None) => Some(format!("应用: {a}")),
            (None, Some(t)) => Some(format!("窗口: {t}")),
            _ => None,
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::AppContext;
    use std::process::Command;

    pub fn detect() -> AppContext {
        let display_server = if std::env::var("WAYLAND_DISPLAY").is_ok() {
            Some("wayland".to_string())
        } else if std::env::var("DISPLAY").is_ok() {
            Some("x11".to_string())
        } else { None };

        // X11: xdotool
        if display_server.as_deref() == Some("x11") {
            if let Ok(out) = Command::new("xdotool")
                .args(["getactivewindow", "getwindowname"]).output() {
                let title = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let app = Command::new("xdotool")
                    .args(["getactivewindow", "getwindowclassname"])
                    .output().ok()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .filter(|s| !s.is_empty());
                return AppContext {
                    app, window_title: Some(title).filter(|s| !s.is_empty()),
                    display_server,
                };
            }
        }

        // Wayland (sway/Hyprland)
        if let Ok(out) = Command::new("hyprctl").args(["activewindow", "-j"]).output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                let app = json_get(&s, "class");
                let title = json_get(&s, "title");
                return AppContext { app, window_title: title, display_server };
            }
        }
        if let Ok(out) = Command::new("swaymsg").args(["-t", "get_tree"]).output() {
            if out.status.success() {
                // 简化：仅返回 display_server
                let _ = out;
            }
        }
        AppContext { display_server, ..Default::default() }
    }

    #[allow(dead_code)]
    fn json_get(s: &str, key: &str) -> Option<String> {
        // 最简 JSON 字段提取（仅供 hyprctl 输出，不依赖 serde_json）
        let pat = format!("\"{key}\":");
        let i = s.find(&pat)?;
        let rest = &s[i + pat.len()..];
        let rest = rest.trim_start();
        if !rest.starts_with('"') { return None; }
        let rest = &rest[1..];
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::AppContext;
    use std::process::Command;

    pub fn detect() -> AppContext {
        // PowerShell: get foreground window title + process name
        let ps = r#"
$h = (Add-Type -MemberDefinition '[DllImport("user32.dll")]public static extern IntPtr GetForegroundWindow();' -Name U -Namespace W -PassThru)::GetForegroundWindow()
$pid2 = 0
Add-Type -MemberDefinition '[DllImport("user32.dll")]public static extern int GetWindowThreadProcessId(IntPtr hWnd, out int lpdwProcessId);' -Name U2 -Namespace W2 -PassThru | Out-Null
[W2.U2]::GetWindowThreadProcessId($h, [ref]$pid2) | Out-Null
$p = Get-Process -Id $pid2 -ErrorAction SilentlyContinue
Write-Output ($p.ProcessName + "|" + $p.MainWindowTitle)
"#;
        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout);
            let s = s.trim();
            let mut parts = s.splitn(2, '|');
            let app = parts.next().map(|x| x.trim().to_string()).filter(|x| !x.is_empty());
            let title = parts.next().map(|x| x.trim().to_string()).filter(|x| !x.is_empty());
            return AppContext { app, window_title: title, display_server: Some("windows".into()) };
        }
        AppContext { display_server: Some("windows".into()), ..Default::default() }
    }
}
