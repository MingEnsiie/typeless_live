# IBus 集成（P2 #25 - skeleton）

## 思路

IBus engine 用 Python 实现：通过 `ibus` Python 绑定注册一个 engine，
快捷键触发后通过 Unix socket 调用 typeless-cli 的 IPC，再 `commit_text` 回 IBus。

## 安装

```bash
sudo apt install python3-ibus python3-gi
mkdir -p ~/.local/share/ibus/component
cp integrations/ibus/typeless.xml ~/.local/share/ibus/component/
chmod +x integrations/ibus/typeless-engine.py
ibus restart
```

在 IBus 设置中添加 "Typeless" 输入法。后台运行：

```bash
typeless-cli run --ipc &
```

## 局限

- 当前为 skeleton。仅在按下 trigger 时调用 `toggle`；事件流回 commit 文本是简化的同步实现。
- 真正生产级建议 fork `ibus-typing-booster` 模板。
