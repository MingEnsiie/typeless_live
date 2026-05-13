# 架构设计

```
[Hotkey] → [Audio cpal+VAD] → [ASR Whisper/Mock] → [LLM Pipeline] → [TextInjector]
                                                          ↑
                                       [Storage SQLite + keyring]
                                       [Context detector]
                                       [Dictionary + Prompt mode]
```

## 状态机

`Idle → Recording → Transcribing → Refining → Injecting → Idle`

任意环节失败 → `Error → Idle`，并通过 `StatusEvent` 广播给 UI。

## Crate 职责

| Crate | 职责 |
|-------|------|
| typeless-storage | 配置(TOML)、SQLite、keyring |
| typeless-audio | cpal 录音、重采样、WebRTC VAD |
| typeless-asr | AsrEngine trait + Whisper/Mock |
| typeless-llm | LlmProvider trait + DeepSeek/MiMo/OpenAI 兼容/Mock + PostProcessor |
| typeless-inject | TextInjector trait + Linux clipboard+paste |
| typeless-hotkey | global-hotkey 封装 |
| typeless-context | 活动应用检测（X11/Wayland） |
| typeless-core | Engine 状态机 + Pipeline 编排 |
| typeless-cli | 命令行守护进程 + 配置/词典/历史 工具 |

## 验收测试

```bash
cargo test --workspace --lib
cargo run --bin typeless-cli -- selftest
cargo run --bin typeless-cli -- refine --mock --text "嗯 啊 你好"
```
