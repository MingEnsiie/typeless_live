// Tauri desktop entry. Build requires webkit2gtk-4.1-dev (Linux),
// 详见 docs/build-desktop.md。当前仓库默认通过 typeless-cli 验证核心功能。
fn main() {
    typeless_desktop_lib::run();
}
