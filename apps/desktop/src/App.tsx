import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { SettingsPage, HistoryPage, DictPage, ModelsPage, AboutPage } from "./pages";

type Status = "idle"|"recording"|"transcribing"|"refining"|"injecting"|"error";
type Tab = "home"|"settings"|"history"|"dict"|"models"|"about";

interface Settings {
  asr: { backend: string; model: string; language: string; translate: boolean };
  llm: { provider: string; model: string; base_url?: string; api_key?: string; temperature: number; max_tokens: number };
  hotkey: { trigger: string; mode: string };
  ui: { show_floating: boolean; theme: string };
  privacy: { local_only: boolean; no_history: boolean; keep_audio: boolean };
  prompt_mode: string; language: string;
}

// ── App shell ──────────────────────────────────────────
function App() {
  const [tab, setTab] = useState<Tab>("home");
  const [isFloat, setIsFloat] = useState(false);
  useEffect(() => { setIsFloat(getCurrentWindow().label === "floating"); }, []);
  if (isFloat) return <FloatingOverlay />;
  const nav: {id:Tab; icon:string; label:string}[] = [
    {id:"home",    icon:"🎙", label:"主页"},
    {id:"history", icon:"📜", label:"历史"},
    {id:"dict",    icon:"📖", label:"词典"},
    {id:"models",  icon:"📦", label:"模型"},
    {id:"settings",icon:"⚙️", label:"设置"},
    {id:"about",   icon:"ℹ️", label:"关于"},
  ];
  return (
    <div className="app">
      <aside className="side">
        <h1>✨ Typeless</h1>
        <nav className="nav">
          {nav.map(n => (
            <button key={n.id} className={tab===n.id?"active":""} onClick={()=>setTab(n.id)}>
              {n.icon} {n.label}
            </button>
          ))}
        </nav>
      </aside>
      <main className="main">
        {tab==="home"     && <HomePage />}
        {tab==="settings" && <SettingsPage />}
        {tab==="history"  && <HistoryPage />}
        {tab==="dict"     && <DictPage />}
        {tab==="models"   && <ModelsPage />}
        {tab==="about"    && <AboutPage />}
      </main>
    </div>
  );
}

function FloatingOverlay() {
  const [status, setStatus] = useState<Status>("idle");
  const [text, setText] = useState("");
  useEffect(() => {
    const u1 = listen<{status:Status}>("status", e => setStatus(e.payload.status));
    const u2 = listen<{text:string}>("partial", e => setText(e.payload.text));
    return () => { u1.then(f=>f()); u2.then(f=>f()); };
  }, []);
  if (status === "idle") return null;
  const labels: Record<string, string> = {
    recording:"录音中…", transcribing:"转写中…", refining:"AI 改写…", injecting:"注入中…"
  };
  return (
    <div className="float">
      <div className="pill">
        <span className={`dot ${status}`}/>
        <span style={{fontSize:13}}>{labels[status]||status}</span>
        {status==="recording" && <span className="wave"><span/><span/><span/><span/><span/></span>}
        {text && <span style={{fontSize:12,color:"#9aa3b2"}}>{text.slice(0,30)}…</span>}
      </div>
    </div>
  );
}

// ── Home Page ───────────────────────────────────────────
function HomePage() {
  const [status, setStatus] = useState<Status>("idle");
  const [finalText, setFinalText] = useState("");
  const [partialText, setPartialText] = useState("");
  useEffect(() => {
    const uns = [
      listen<{status:Status}>("status", e => { setStatus(e.payload.status); if(e.payload.status==="idle") setPartialText(""); }),
      listen<{text:string}>("partial", e => setPartialText(e.payload.text)),
      listen<{text:string}>("final",   e => { setFinalText(e.payload.text); setPartialText(""); }),
    ];
    return () => { uns.forEach(p => p.then(f=>f())); };
  }, []);

  const toggle = () => invoke("toggle_recording").catch(console.error);
  const copy   = () => finalText && navigator.clipboard?.writeText(finalText);
  const stMap: Record<string,string> = {
    idle:"待机，按快捷键或点击按钮开始",
    recording:"🔴 录音中…  再次点击停止",
    transcribing:"⏳ 转写中…",
    refining:"✨ AI 润色中…",
    injecting:"⌨️ 注入文本…",
    error:"⚠️ 出错",
  };
  return (
    <>
      <h2>语音输入</h2>
      <div className="card center">
        <button className={`mic-btn ${status==="recording"?"recording":""}`} onClick={toggle}>
          <span style={{fontSize:"2.5rem"}}>{status==="recording"?"⏹":"🎙"}</span>
        </button>
        <p style={{marginTop:12,color:"#9aa3b2",fontSize:13}}>{stMap[status]||status}</p>
        {partialText && <p style={{marginTop:8,color:"#ccc",fontStyle:"italic",fontSize:13}}>{partialText}</p>}
      </div>
      {finalText && (
        <div className="card">
          <div style={{display:"flex",justifyContent:"space-between",alignItems:"center",marginBottom:8}}>
            <span style={{fontWeight:600,fontSize:14}}>转写结果</span>
            <button className="btn-sm" onClick={copy}>复制</button>
          </div>
          <p style={{lineHeight:1.7,fontSize:15,whiteSpace:"pre-wrap"}}>{finalText}</p>
        </div>
      )}
    </>
  );
}
