//! Unix-domain-socket IPC 服务器：供 fcitx5 / ibus addon 与 typeless 守护进程通信。
//!
//! 协议：行分隔 JSON。
//! 客户端 → 服务端：
//!   {"cmd":"toggle"}            切换录音
//!   {"cmd":"start"}             开始录音
//!   {"cmd":"stop"}              停止并处理
//!   {"cmd":"status"}            查询状态
//!   {"cmd":"refine","text":..}  仅 LLM 后处理
//!
//! 服务端 → 客户端（事件广播 + 命令回执）：
//!   {"event":"state","value":"Recording"}
//!   {"event":"final","text":"..."}
//!   {"ok":true}
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use typeless_core::engine::Engine;

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "lowercase")]
enum Req {
    Toggle,
    Start,
    Stop,
    Status,
    Refine { text: String },
}

#[derive(Debug, Serialize)]
struct OkResp {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<String>,
}

pub fn socket_path() -> PathBuf {
    let runtime = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/tmp/runtime-{}", std::env::var("USER").unwrap_or_default()));
    PathBuf::from(runtime).join("typeless.sock")
}

pub async fn serve(engine: Arc<Engine>) -> Result<()> {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let listener = UnixListener::bind(&path)?;
    tracing::info!("IPC server listening on {}", path.display());
    loop {
        let (stream, _) = listener.accept().await?;
        let engine = engine.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, engine).await {
                tracing::warn!("ipc client error: {e}");
            }
        });
    }
}

async fn handle_client(stream: UnixStream, engine: Arc<Engine>) -> Result<()> {
    let (rd, mut wr) = stream.into_split();
    let mut lines = BufReader::new(rd).lines();

    // 订阅引擎事件并 push 给客户端
    let mut ev_rx = engine.subscribe();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(16);
    tokio::spawn(async move {
        while let Ok(ev) = ev_rx.recv().await {
            if let Ok(s) = serde_json::to_string(&ev) {
                if tx.send(s).await.is_err() {
                    break;
                }
            }
        }
    });

    loop {
        tokio::select! {
            line = lines.next_line() => {
                let line = match line { Ok(Some(l)) => l, _ => break };
                if line.trim().is_empty() { continue; }
                let resp = match serde_json::from_str::<Req>(&line) {
                    Ok(req) => handle_req(req, &engine).await,
                    Err(e) => OkResp { ok: false, text: Some(format!("parse error: {e}")), state: None },
                };
                let s = serde_json::to_string(&resp)?;
                wr.write_all(s.as_bytes()).await?;
                wr.write_all(b"\n").await?;
            }
            Some(ev) = rx.recv() => {
                wr.write_all(ev.as_bytes()).await?;
                wr.write_all(b"\n").await?;
            }
        }
    }
    Ok(())
}

async fn handle_req(req: Req, engine: &Arc<Engine>) -> OkResp {
    match req {
        Req::Toggle => {
            if engine.is_recording() {
                match engine.stop_and_process().await {
                    Ok(t) => OkResp { ok: true, text: t, state: None },
                    Err(e) => OkResp { ok: false, text: Some(e.to_string()), state: None },
                }
            } else {
                match engine.start_recording() {
                    Ok(_) => OkResp { ok: true, text: None, state: Some("Recording".into()) },
                    Err(e) => OkResp { ok: false, text: Some(e.to_string()), state: None },
                }
            }
        }
        Req::Start => match engine.start_recording() {
            Ok(_) => OkResp { ok: true, text: None, state: Some("Recording".into()) },
            Err(e) => OkResp { ok: false, text: Some(e.to_string()), state: None },
        },
        Req::Stop => match engine.stop_and_process().await {
            Ok(t) => OkResp { ok: true, text: t, state: None },
            Err(e) => OkResp { ok: false, text: Some(e.to_string()), state: None },
        },
        Req::Status => OkResp {
            ok: true,
            text: None,
            state: Some(if engine.is_recording() { "Recording" } else { "Idle" }.into()),
        },
        Req::Refine { text: _ } => OkResp {
            ok: false,
            text: Some("refine via IPC: not yet implemented".into()),
            state: None,
        },
    }
}
