# Typeless 输入法集成

本目录提供与 Linux 输入法框架的集成骨架，让用户在 fcitx5 / ibus 中直接通过快捷键唤起 typeless 录音。

## 架构

```
[fcitx5/ibus addon]  --IPC(UDS JSON)-->  [typeless-cli run --ipc]
                                              |
                                              v
                                       Engine (ASR + LLM)
                                              |
                                       Inject text back via addon's
                                       commit_string() / forward_text
```

`typeless-cli run` 已内置一个 Unix-socket IPC 服务器（`$XDG_RUNTIME_DIR/typeless.sock`），
addon 只需写少量胶水代码：

- 接收 fcitx5/ibus 的快捷键事件
- 通过 socket 发送 `{"cmd":"toggle"}`
- 监听 `final_text` 事件 → `commit_string(text)`

## 子目录

- `fcitx5/`  - fcitx5 addon skeleton（C++ 或 Lua/Python via fcitx5 plugin loader）
- `ibus/`    - IBus engine skeleton（Python）

完整实现留作后续工作；当前提供：可工作的 IPC 协议 + 配置/manifest 文件 + 详细 README。
