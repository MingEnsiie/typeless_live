# systemd --user 集成

将 `typeless.service` 拷贝到 `~/.config/systemd/user/`：

```bash
mkdir -p ~/.config/systemd/user
cp packaging/systemd/typeless.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now typeless.service
systemctl --user status typeless.service
journalctl --user -u typeless.service -f
```

停止 / 卸载：

```bash
systemctl --user disable --now typeless.service
rm ~/.config/systemd/user/typeless.service
```
