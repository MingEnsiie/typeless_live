# Typeless Live

100% 复刻 Wispr Flow 风格的 AI 语音输入法。

> 按住快捷键 → 说话 → 自动转写 → LLM 改写 → 注入到当前焦点应用

## 特性

- ✅ 跨平台核心：Rust + Tauri，一次实现，多端复用
- ✅ 本地 ASR：whisper.cpp，离线可用
- ✅ 多 LLM Provider：DeepSeek / 小米 MiMo / 本地 llama.cpp
- ✅ 智能后处理：去口癖、加标点、上下文风格切换
- ✅ Linux 原生 IME 集成（Fcitx5 / IBus）
- ✅ 隐私优先：录音不出本机模式、keyring 加密密钥

## 快速开始

```bash
# 1. 编译核心
cargo build --release

# 2. 启动 CLI 守护进程（最小 demo）
cargo run --bin typeless-cli -- run

# 3. 配置 API Key（DeepSeek 示例）
typeless-cli config set llm.provider deepseek
typeless-cli config set llm.api_key sk-xxx

# 4. 桌面 UI（需 Node.js）
cd apps/desktop && npm install && npm run tauri dev
```

## 项目结构

```
crates/
  typeless-core/       业务编排
  typeless-audio/      cpal 录音 + VAD
  typeless-asr/        Whisper ASR
  typeless-llm/        LLM Provider 抽象
  typeless-inject/     文本注入
  typeless-storage/    SQLite + 配置
  typeless-hotkey/     全局热键
  typeless-context/    应用上下文
  typeless-cli/        命令行入口
apps/
  desktop/             Tauri 桌面应用
```

## License

MIT OR Apache-2.0
