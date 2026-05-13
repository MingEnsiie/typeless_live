# Fcitx5 集成（P2 #24 - skeleton）

## 思路

Fcitx5 addon 通过 typeless 守护进程的 Unix-socket IPC 控制录音并接收最终文本。

最简方案：使用 `fcitx5-lua` 插件 + 几行 Lua 脚本，无需编译 C++ 模块。

## 启用前提

1. 安装 fcitx5 与 fcitx5-lua：
   ```
   sudo apt install fcitx5 fcitx5-module-lua
   ```
2. 后台运行 typeless 守护进程并启用 IPC：
   ```
   typeless-cli run --ipc
   ```
   sock 路径默认 `$XDG_RUNTIME_DIR/typeless.sock`

## 安装 Lua 脚本

```bash
mkdir -p ~/.local/share/fcitx5/lua/typeless
cp integrations/fcitx5/typeless.lua ~/.local/share/fcitx5/lua/typeless/extension.lua
fcitx5-remote -r
```

在 fcitx5 配置中将 `Toggle Typeless` 快捷键绑定到自定义按键（默认 `Ctrl+Alt+Space`，可在 `extension.lua` 顶部修改）。

## 局限

- 当前 Lua 仅触发 toggle 命令；事件回执（final text）需要异步 socket，fcitx5-lua API 受限。
- 完整 C++ addon 见 `addon.conf` 与 `typeless-fcitx5/` 模板（留作 TODO）。
