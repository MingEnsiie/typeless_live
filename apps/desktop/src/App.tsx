import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

type Status = "idle" | "recording" | "transcribing" | "refining" | "injecting" | "error";
type Tab = "home" | "settings" | "history" | "dict" | "models" | "about";

interface Settings {
  asr: { backend: string; model: string; language: string; translate: boolean };
  llm: { provider: string; model: string; base_url?: string; api_key?: string; temperature: number; max_tokens: number };
  hotkey: { trigger: string; mode: string };
  ui: { show_floating: boolean; theme: string };
  privacy: { local_only: boolean; no_history: boolean; keep_audio: boolean };
  prompt_mode: string;
  language: string;
}

interface HistoryRec { id: string; created_at: string; raw_text: string; final_text: string; mode: string; app?: string }
interface DictEntry { id: number; from_text: string; to_text: string; note?: string }

function FloatingOverlay() {
  const [status, setStatus] = useState<Status>("idle");
  const [text, setText] = useState("");
  useEffect(() => {
    const u1 = listen<{ status: Status }>("status", (e) => setStatus(e.payload.status));
    const u2 = listen<{ text: string }>("partial", (e) => setText(e.payload.text));
    return () => { u1.then(f => f()); u2.then(f => f()); };
  }, []);
  if (status === "idle") return null;
  return (
    <div className="float">
      <div className="pill">
        <span className={`dot ${status}`}></span>
        <span style={{ fontSize: 13 }}>
          {status === "recording" && "正在录音..."}
          {status === "transcribing" && "转写中..."}
          {status === "refining" && "AI 改写中..."}
          {status === "injecting" && "注入中..."}
        </span>
        {status === "recording" && (
          <span className="wave"><span></span><span></span><span></span><span></span><span></span></span>
        )}
        {text && <span style={{ fontSize: 12, color: "#9aa3b2" }}>{text.slice(0, 30)}...</span>}
      </div>
    </div>
  );
}

function App() {
  const [tab, setTab] = useState<Tab>("home");
  const [isFloat, setIsFloat] = useState(false);

  useEffect(() => {
    getCurrentWindow().label === "floating" ? setIsFloat(true) : setIsFloat(false);
  }, []);

  if (isFloat) return <FloatingOverlay />;

  return (
    <div className="app">
      <aside className="side">
        <h1>✨ Typeless</h1>
        <nav className="nav">
          <button className={tab === "home" ? "active" : ""} onClick={() => setTab("home")}>🏠 主页</button>
          <button className={tab === "history" ? "active" : ""} onClick={() => setTab("history")}>📜 历史</button>
          <button className={tab === "dict" ? "active" : ""} onClick={() => setTab("dict")}>📖 词典</button>
          <button className={tab === "models" ? "active" : ""} onClick={() => setTab("models")}>📦 模型</button>
          <button className={tab === "settings" ? "active" : ""} onClick={() => setTab("settings")}>⚙️ 设置</button>
          <button className={tab === "about" ? "active" : ""} onClick={() => setTab("about")}>ℹ️ 关于</button>
        </nav>
      </aside>
      <main className="main">
        {tab === "home" && <HomePage />}
        {tab === "settings" && <SettingsPage />}
        {tab === "history" && <HistoryPage />}
        {tab === "dict" && <DictPage />}
        {tab === "models" && <ModelsPage />}
        {tab === "about" && <AboutPage />}
      </main>
    </div>
  );
}

function HomePage() {
  const [status, setStatus] = useState<Status>("idle");
  const [output, setOutput] = useState("");

  useEffect(() => {
    const u1 = listen<{ status: Status }>("status", e => setStatus(e.payload.status));
    const u2 = listen<{ text: string }>("final", e => setOutput(e.payload.text));
    return () => { u1.then(f => f()); u2.then(f => f()); };
  }, []);

  const toggle = async () => { try { await invoke("toggle_recording"); } catch (e) { alert(e); } };

  return (
    <>
      <h2>主页</h2>
      <div className="card">
        <div className="row">
          <span className="status-pill"><span className={`dot ${status}`}></span> {status}</span>
          <button className="btn" onClick={toggle}>
            {status === "idle" ? "🎙 开始录音" : "⏹ 停止"}
          </button>
        </div>
        <div style={{ marginTop: 12, color: "#9aa3b2", fontSize: 13 }}>
          快捷键：Ctrl+Alt+Space（按一下开始，再按一下结束）
        </div>
      </div>
      {output && (
        <div className="card">
          <h3>最近输出</h3>
          <p style={{ marginTop: 8 }}>{output}</p>
        </div>
      )}
    </>
  );
}

function SettingsPage() {
  const [s, setS] = useState<Settings | null>(null);
  useEffect(() => { invoke<Settings>("get_settings").then(setS).catch(console.error); }, []);
  if (!s) return <div>加载中...</div>;

  const save = async () => {
    try { await invoke("save_settings", { settings: s }); alert("已保存"); }
    catch (e) { alert("保存失败: " + e); }
  };
  const update = (path: string, value: any) => {
    const ns: any = JSON.parse(JSON.stringify(s));
    const parts = path.split(".");
    let o = ns;
    for (let i = 0; i < parts.length - 1; i++) o = o[parts[i]];
    o[parts[parts.length - 1]] = value;
    setS(ns);
  };

  return (
    <>
      <h2>设置</h2>
      <div className="card">
        <h3>LLM</h3>
        <div className="field"><label>Provider</label>
          <select value={s.llm.provider} onChange={e => update("llm.provider", e.target.value)}>
            <option value="deepseek">DeepSeek</option>
            <option value="mimo">小米 MiMo</option>
            <option value="openai">OpenAI 兼容</option>
            <option value="mock">Mock（无需 key）</option>
          </select>
        </div>
        <div className="field"><label>模型</label><input value={s.llm.model} onChange={e => update("llm.model", e.target.value)} /></div>
        <div className="field"><label>Base URL（可选）</label><input value={s.llm.base_url || ""} onChange={e => update("llm.base_url", e.target.value)} placeholder="留空使用默认" /></div>
        <div className="field"><label>API Key</label><input type="password" value={s.llm.api_key || ""} onChange={e => update("llm.api_key", e.target.value)} placeholder="sk-..." /></div>
        <div className="field"><label>Temperature</label><input type="number" step="0.1" value={s.llm.temperature} onChange={e => update("llm.temperature", parseFloat(e.target.value))} /></div>
      </div>
      <div className="card">
        <h3>ASR</h3>
        <div className="field"><label>引擎</label>
          <select value={s.asr.backend} onChange={e => update("asr.backend", e.target.value)}>
            <option value="whisper">Whisper（本地）</option>
            <option value="mock">Mock</option>
          </select>
        </div>
        <div className="field"><label>模型文件</label><input value={s.asr.model} onChange={e => update("asr.model", e.target.value)} /></div>
        <div className="field"><label>语言</label>
          <select value={s.asr.language} onChange={e => update("asr.language", e.target.value)}>
            <option value="auto">自动检测</option>
            <option value="zh">中文</option>
            <option value="en">英文</option>
            <option value="ja">日文</option>
            <option value="ko">韩文</option>
          </select>
        </div>
      </div>
      <div className="card">
        <h3>快捷键 / Prompt</h3>
        <div className="field"><label>触发键</label><input value={s.hotkey.trigger} onChange={e => update("hotkey.trigger", e.target.value)} /></div>
        <div className="field"><label>Prompt 模式</label>
          <select value={s.prompt_mode} onChange={e => update("prompt_mode", e.target.value)}>
            <option value="default">默认</option>
            <option value="email">邮件</option>
            <option value="code">代码</option>
            <option value="formal">正式书面</option>
            <option value="translate_en">翻译为英文</option>
          </select>
        </div>
      </div>
      <div className="card">
        <h3>隐私</h3>
        <div className="field"><label>仅本地（不调用云）</label><input type="checkbox" checked={s.privacy.local_only} onChange={e => update("privacy.local_only", e.target.checked)} /></div>
        <div className="field"><label>不保存历史</label><input type="checkbox" checked={s.privacy.no_history} onChange={e => update("privacy.no_history", e.target.checked)} /></div>
      </div>
      <button className="btn" onClick={save}>💾 保存</button>
    </>
  );
}

function HistoryPage() {
  const [list, setList] = useState<HistoryRec[]>([]);
  const reload = () => invoke<HistoryRec[]>("list_history", { limit: 100 }).then(setList);
  useEffect(() => { reload(); }, []);
  return (
    <>
      <h2>历史记录</h2>
      <div className="card">
        <table>
          <thead><tr><th>时间</th><th>模式</th><th>原文</th><th>改写</th></tr></thead>
          <tbody>
            {list.map(r => (
              <tr key={r.id}>
                <td style={{ fontSize: 12, color: "#9aa3b2" }}>{r.created_at.slice(0, 19).replace("T", " ")}</td>
                <td>{r.mode}</td>
                <td style={{ color: "#9aa3b2", maxWidth: 200 }}>{r.raw_text.slice(0, 40)}</td>
                <td>{r.final_text}</td>
              </tr>
            ))}
            {list.length === 0 && <tr><td colSpan={4} style={{ textAlign: "center", color: "#9aa3b2" }}>暂无记录</td></tr>}
          </tbody>
        </table>
      </div>
    </>
  );
}

function DictPage() {
  const [list, setList] = useState<DictEntry[]>([]);
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const reload = () => invoke<DictEntry[]>("dict_list").then(setList);
  useEffect(() => { reload(); }, []);
  const add = async () => {
    if (!from || !to) return;
    await invoke("dict_add", { from, to, note: null });
    setFrom(""); setTo(""); reload();
  };
  const del = async (id: number) => {
    await invoke("dict_remove", { id }); reload();
  };
  return (
    <>
      <h2>词典</h2>
      <div className="card">
        <div className="row">
          <input value={from} onChange={e => setFrom(e.target.value)} placeholder="原文（口语）" />
          <input value={to} onChange={e => setTo(e.target.value)} placeholder="改写为" />
          <button className="btn" onClick={add}>添加</button>
        </div>
      </div>
      <div className="card">
        <table>
          <thead><tr><th>原文</th><th>改写</th><th>备注</th><th></th></tr></thead>
          <tbody>
            {list.map(e => (
              <tr key={e.id}>
                <td>{e.from_text}</td><td>{e.to_text}</td><td>{e.note || "-"}</td>
                <td><button className="btn ghost" onClick={() => del(e.id)}>删除</button></td>
              </tr>
            ))}
            {list.length === 0 && <tr><td colSpan={4} style={{ textAlign: "center", color: "#9aa3b2" }}>暂无</td></tr>}
          </tbody>
        </table>
      </div>
    </>
  );
}

function ModelsPage() {
  const [models, setModels] = useState<any[]>([]);
  useEffect(() => { invoke<any[]>("list_models").then(setModels).catch(() => setModels([])); }, []);
  const presets = [
    { kind: "whisper", name: "ggml-tiny.bin", url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin", size: "75 MB" },
    { kind: "whisper", name: "ggml-base.bin", url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin", size: "142 MB" },
    { kind: "whisper", name: "ggml-small.bin", url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin", size: "466 MB" },
  ];
  return (
    <>
      <h2>模型市场</h2>
      <div className="card">
        <table>
          <thead><tr><th>类型</th><th>名称</th><th>大小</th><th>状态</th><th></th></tr></thead>
          <tbody>
            {presets.map(m => {
              const installed = models.some(x => x.name === m.name);
              return (
                <tr key={m.name}>
                  <td>{m.kind}</td><td>{m.name}</td><td>{m.size}</td>
                  <td>{installed ? "✅ 已安装" : "未下载"}</td>
                  <td>{!installed && <button className="btn" onClick={() => invoke("download_model", { kind: m.kind, name: m.name, url: m.url }).then(() => alert("下载完成")).catch(e => alert(e))}>下载</button>}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </>
  );
}

function AboutPage() {
  return (
    <>
      <h2>关于 Typeless</h2>
      <div className="card">
        <p>AI 语音输入法 · 复刻 Wispr Flow 风格。</p>
        <p style={{ marginTop: 12, color: "#9aa3b2", fontSize: 13 }}>
          源码 / Issues：<a style={{ color: "#22d3ee" }} href="https://github.com/MingEnsiie/typeless_live" target="_blank">GitHub</a>
        </p>
      </div>
    </>
  );
}

export default App;
