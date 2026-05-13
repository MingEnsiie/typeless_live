-- Typeless fcitx5 集成（最小骨架）
-- 通过 socket(AF_UNIX) 与 typeless-cli run --ipc 通信。
-- 由于 fcitx5-lua 沙盒不支持 raw sockets，这里改为调用 socat / nc 命令行管道。

local fcitx = require("fcitx")

local SOCK = (os.getenv("XDG_RUNTIME_DIR") or "/tmp") .. "/typeless.sock"

local function send(json)
    -- 使用 ncat (nmap-ncat) 或 socat 与 unix socket 通信
    local cmd = string.format("printf '%%s\\n' '%s' | ncat -U %q 2>/dev/null", json, SOCK)
    local f = io.popen(cmd, "r")
    if not f then return nil end
    local resp = f:read("*l")
    f:close()
    return resp
end

local function toggle()
    local resp = send([[{"cmd":"toggle"}]])
    if resp then
        -- 若 stop 返回了 final text，则提交到当前输入位置
        local text = resp:match('"text":"([^"]*)"')
        if text and text ~= "" then
            -- fcitx.commitString 会把 text 直接 commit 到目标应用
            fcitx.commitString(text)
        end
    end
    return ""
end

-- 注册快捷键 Ctrl+Alt+Space → toggle 录音
fcitx.addQuickPhraseHandler(function(text)
    if text == "typeless" then
        return toggle()
    end
    return nil
end)

ime.register_command("typeless", toggle, "Typeless toggle", "cmd",
                     "Trigger typeless voice input (toggle)")
