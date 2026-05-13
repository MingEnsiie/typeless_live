#!/usr/bin/env python3
"""
Typeless IBus engine skeleton (P2 #25).

通过 Unix socket 与 `typeless-cli run --ipc` 守护进程通信。
按下绑定的快捷键 → toggle → 收到 final 文本 → commit_text 回应用。

依赖：
    sudo apt install python3-ibus python3-gi
"""

import json
import os
import socket
import sys
import threading

import gi

gi.require_version("IBus", "1.0")
from gi.repository import GLib, IBus  # noqa: E402

SOCK = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), "typeless.sock")


class TypelessEngine(IBus.Engine):
    __gtype_name__ = "TypelessEngine"

    def __init__(self):
        super().__init__()
        self._sock = None
        self._connect()

    def _connect(self):
        try:
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.connect(SOCK)
            self._sock = s
            t = threading.Thread(target=self._listen, daemon=True)
            t.start()
        except OSError as e:
            print(f"typeless: cannot connect to {SOCK}: {e}", file=sys.stderr)

    def _listen(self):
        f = self._sock.makefile("r")
        for line in f:
            try:
                obj = json.loads(line)
            except Exception:
                continue
            text = obj.get("text") or ""
            if obj.get("event") == "final" and text:
                GLib.idle_add(self._commit, text)
            elif obj.get("ok") and text:
                GLib.idle_add(self._commit, text)

    def _commit(self, text):
        self.commit_text(IBus.Text.new_from_string(text))
        return False

    def _send(self, cmd):
        if not self._sock:
            self._connect()
        if self._sock:
            try:
                self._sock.sendall((json.dumps(cmd) + "\n").encode())
            except OSError:
                self._sock = None

    def do_process_key_event(self, keyval, keycode, state):
        # Ctrl+Alt+Space => toggle
        if (
            state & IBus.ModifierType.CONTROL_MASK
            and state & IBus.ModifierType.MOD1_MASK
            and keyval == IBus.KEY_space
        ):
            self._send({"cmd": "toggle"})
            return True
        return False


def main():
    bus = IBus.Bus()
    if not bus.is_connected():
        print("typeless: ibus daemon not running", file=sys.stderr)
        sys.exit(1)
    factory = IBus.Factory.new(bus.get_connection())
    factory.add_engine("typeless", GLib.Type.from_name("TypelessEngine"))
    if "--ibus" in sys.argv:
        bus.request_name("org.freedesktop.IBus.Typeless", 0)
    GLib.MainLoop().run()


if __name__ == "__main__":
    main()
