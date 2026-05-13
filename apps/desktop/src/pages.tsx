// pages.tsx — SettingsPage, HistoryPage, DictPage, ModelsPage, AboutPage
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Settings {
  asr: { backend: string; model: string; language: string; translate: boolean };
  llm: { provider: string; model: string; base_url?: string; api_key?: string; temperature: number; max_tokens: number };
  hotkey: { trigger: string; mode: string };
  ui: { show_floating: boolean; theme: string };
  privacy: { local_only: boolean; no_history: boolean; keep_audio: boolean };
  prompt_mode: string; language: string;
}
interface HistoryRec { id: string; created_at: string; raw_text: string; final_text: string; mode: string }
interface DictEntry { id: number; from_text: string; to_text: string; note?: string }
interface ModelInfo { kind: string; name: string; path: string; size_bytes: number; downloaded: boolean }
interface ProviderPreset { id: string; name: string; base_url: string; default_model: string }
interface PingResult { ok: boolean; ms: number; reply: string }
interface DownloadProgress { name: string; downloaded: number; total: number; pct: number }

export function SettingsPage() {
  const [s, setS] = useState<Settings|null>(null);
  const [presets, setPresets] = useState<ProviderPreset[]>([]);
  const [ping, setPing] = useState<PingResult|null>(null);
  const [pinging, setPinging] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    invoke<Settings>("get_settings").then(setS).catch(console.error);
    invoke<ProviderPreset[]>("get_providers").then(setPresets).catch(console.error);
  }, []);
  if (!s) return <div style={{padding:24,color:"#888"}}>加载中…</div>;

  const update = (path: string, value: unknown) => {
    const ns = JSON.parse(JSON.stringify(s)) as Record<string,unknown>;
    const parts = path.split(".");
    let o: Record<string,unknown> = ns;
    for (let i=0;i<parts.length-1;i++) o = o[parts[i]] as Record<string,unknown>;
    o[parts[parts.length-1]] = value;
    setS(ns as unknown as Settings);
  };

  const selectPreset = (id: string) => {
    const p = presets.find(x=>x.id===id);
    if (!p) return;
    update("llm.provider", id);
    if (p.base_url) update("llm.base_url", p.base_url);
    update("llm.model", p.default_model);
  };

  const save = async () => {
    try { await invoke("save_settings",{settings:s}); setSaved(true); setTimeout(()=>setSaved(false),2000); }
    catch(e) { alert("保存失败: "+e); }
  };

  const doPing = async () => {
    await save();
    setPinging(true); setPing(null);
    try { const r = await invoke<PingResult>("ping_llm"); setPing(r); }
    catch(e) { setPing({ok:false,ms:0,reply:String(e)}); }
    finally { setPinging(false); }
  };

  return (
    <>
      <h2>设置</h2>
      <div className="card">
        <h3>🤖 大语言模型 (API Key)</h3>
        <div className="field"><label>Provider</label>
          <select value={s.llm.provider} onChange={e=>selectPreset(e.target.value)}>
            {presets.map(p=><option key={p.id} value={p.id}>{p.name}</option>)}
          </select>
        </div>
        <div className="field"><label>模型名称</label>
          <input value={s.llm.model} onChange={e=>update("llm.model",e.target.value)} placeholder="mimo-v2.5-pro"/>
        </div>
        <div className="field"><label>Base URL</label>
          <input value={s.llm.base_url||""} onChange={e=>update("llm.base_url",e.target.value)}/>
        </div>
        <div className="field"><label>API Key</label>
          <input type="password" value={s.llm.api_key||""} onChange={e=>update("llm.api_key",e.target.value)} placeholder="tp-… 或 sk-…"/>
        </div>
        <div style={{marginTop:12,display:"flex",alignItems:"center",gap:12}}>
          <button className="btn-sm" onClick={doPing} disabled={pinging}>{pinging?"测试中…":"🔌 测试连接"}</button>
          {ping && <span style={{fontSize:13,color:ping.ok?"#30d158":"#ff453a"}}>
            {ping.ok?`✓ ${ping.ms}ms · ${ping.reply.slice(0,30)}`:`✗ ${ping.reply.slice(0,60)}`}
          </span>}
        </div>
      </div>
      <div className="card">
        <h3>🎙 语音识别 (ASR)</h3>
        <div className="field"><label>引擎</label>
          <select value={s.asr.backend} onChange={e=>update("asr.backend",e.target.value)}>
            <option value="whisper">Whisper（本地）</option>
            <option value="mock">Mock（仅测试）</option>
          </select>
        </div>
        <div className="field"><label>模型文件名</label>
          <input value={s.asr.model} onChange={e=>update("asr.model",e.target.value)} placeholder="ggml-base.bin"/>
        </div>
        <div className="field"><label>语言</label>
          <select value={s.asr.language} onChange={e=>update("asr.language",e.target.value)}>
            <option value="auto">自动</option><option value="zh">中文</option>
            <option value="en">English</option><option value="ja">日本語</option>
          </select>
        </div>
      </div>
      <div className="card">
        <h3>⌨️ 快捷键 / Prompt</h3>
        <div className="field"><label>触发键</label>
          <input value={s.hotkey.trigger} onChange={e=>update("hotkey.trigger",e.target.value)}/>
        </div>
        <div className="field"><label>Prompt 模式</label>
          <select value={s.prompt_mode} onChange={e=>update("prompt_mode",e.target.value)}>
            <option value="default">默认</option><option value="email">邮件</option>
            <option value="code">代码</option><option value="formal">正式</option>
            <option value="translate_en">翻译→英文</option>
          </select>
        </div>
      </div>
      <div className="card">
        <h3>🔒 隐私</h3>
        <div className="field"><label>仅本地（不调用云端）</label>
          <input type="checkbox" checked={s.privacy.local_only} onChange={e=>update("privacy.local_only",e.target.checked)}/>
        </div>
        <div className="field"><label>不保存历史</label>
          <input type="checkbox" checked={s.privacy.no_history} onChange={e=>update("privacy.no_history",e.target.checked)}/>
        </div>
      </div>
      <button className="btn" onClick={save}>{saved?"✓ 已保存":"💾 保存设置"}</button>
    </>
  );
}

export function HistoryPage() {
  const [list, setList] = useState<HistoryRec[]>([]);
  useEffect(() => { invoke<HistoryRec[]>("list_history",{limit:50}).then(setList).catch(console.error); },[]);
  return (
    <>
      <h2>历史记录</h2>
      {list.length===0 && <p style={{color:"#888",padding:16}}>暂无记录</p>}
      {list.map(r => (
        <div key={r.id} className="card hist-item" onClick={()=>navigator.clipboard?.writeText(r.final_text||r.raw_text)}>
          <div style={{fontSize:12,color:"#666",marginBottom:4}}>{r.created_at} · {r.mode}</div>
          <div style={{fontSize:14,color:"#ddd",marginBottom:4,fontStyle:"italic",color:"#888"}}>{r.raw_text.slice(0,60)}</div>
          <div style={{fontSize:15,lineHeight:1.6}}>{r.final_text||r.raw_text}</div>
        </div>
      ))}
    </>
  );
}

export function DictPage() {
  const [list, setList] = useState<DictEntry[]>([]);
  const [from, setFrom] = useState(""); const [to, setTo] = useState(""); const [note, setNote] = useState("");
  const reload = () => invoke<DictEntry[]>("dict_list").then(setList).catch(console.error);
  useEffect(() => { reload(); },[]);
  const add = async () => {
    if (!from||!to) return;
    await invoke("dict_add",{from,to,note:note||null});
    setFrom(""); setTo(""); setNote(""); reload();
  };
  return (
    <>
      <h2>自定义词典</h2>
      <div className="card">
        <div style={{display:"flex",gap:8,flexWrap:"wrap"}}>
          <input placeholder="原词" value={from} onChange={e=>setFrom(e.target.value)} style={{flex:1}}/>
          <input placeholder="替换为" value={to} onChange={e=>setTo(e.target.value)} style={{flex:1}}/>
          <input placeholder="备注（可选）" value={note} onChange={e=>setNote(e.target.value)} style={{flex:1}}/>
          <button className="btn-sm" onClick={add}>添加</button>
        </div>
      </div>
      {list.map(e=>(
        <div key={e.id} className="card" style={{display:"flex",justifyContent:"space-between",alignItems:"center"}}>
          <span style={{fontSize:14}}>{e.from_text} → <b>{e.to_text}</b>{e.note&&<span style={{color:"#888",fontSize:12}}> · {e.note}</span>}</span>
          <button className="btn-sm danger" onClick={()=>invoke("dict_remove",{id:e.id}).then(reload)}>删除</button>
        </div>
      ))}
    </>
  );
}

export function ModelsPage() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [progress, setProgress] = useState<Record<string,number>>({});
  const [downloading, setDownloading] = useState<Record<string,boolean>>({});
  const reload = () => invoke<ModelInfo[]>("list_models").then(setModels).catch(console.error);
  useEffect(() => {
    reload();
    const u = listen<DownloadProgress>("download_progress", e => {
      setProgress(p=>({...p,[e.payload.name]:e.payload.pct}));
    });
    return () => { u.then(f=>f()); };
  },[]);

  const download = async (name: string) => {
    setDownloading(d=>({...d,[name]:true}));
    setProgress(p=>({...p,[name]:0}));
    try { await invoke("download_model",{name}); reload(); }
    catch(e) { alert("下载失败: "+e); }
    finally { setDownloading(d=>({...d,[name]:false})); }
  };

  const fmt = (b: number) => b>1e9?`${(b/1e9).toFixed(1)}GB`:b>1e6?`${(b/1e6).toFixed(0)}MB`:`${(b/1e3).toFixed(0)}KB`;

  return (
    <>
      <h2>模型管理</h2>
      <p style={{color:"#888",fontSize:13,marginBottom:16}}>Whisper 模型用于本地语音识别。推荐 ggml-base.bin（142MB，速度与精度均衡）。</p>
      {models.map(m=>(
        <div key={m.name} className="card" style={{display:"flex",justifyContent:"space-between",alignItems:"center"}}>
          <div>
            <div style={{fontWeight:600,fontSize:14}}>{m.name}</div>
            <div style={{fontSize:12,color:"#888",marginTop:2}}>{fmt(m.size_bytes)} · {m.kind}</div>
            {downloading[m.name] && <div style={{marginTop:6,fontSize:12,color:"#ffd60a"}}>下载中 {progress[m.name]||0}%</div>}
          </div>
          {m.downloaded
            ? <span style={{color:"#30d158",fontSize:13}}>✓ 已下载</span>
            : <button className="btn-sm" onClick={()=>download(m.name)} disabled={downloading[m.name]}>
                {downloading[m.name]?`${progress[m.name]||0}%…`:"下载"}
              </button>}
        </div>
      ))}
    </>
  );
}

export function AboutPage() {
  return (
    <>
      <h2>关于 Typeless</h2>
      <div className="card">
        <p style={{lineHeight:1.8}}>
          <b>Typeless</b> 是一个 Wispr Flow 风格的 AI 语音输入法。<br/>
          本地 Whisper 实时转写 + 云端大模型（MiMo / DeepSeek / OpenAI）后处理：<br/>
          去除口癖、添加标点、纠正错别字，一键注入到任意输入框。
        </p>
        <br/>
        <p style={{color:"#888",fontSize:13}}>版本 0.1.0 · MIT License</p>
        <p style={{color:"#888",fontSize:13}}>
          <a href="https://github.com/MingEnsiie/typeless_live" style={{color:"#0a84ff"}}>
            GitHub: MingEnsiie/typeless_live
          </a>
        </p>
      </div>
    </>
  );
}
