//! Web UI 服务器：axum HTTP + SSE 实时事件 + REST API。
use axum::{
    extract::State,
    response::{Html, sse::{Event, KeepAlive, Sse}},
    routing::{get, post},
    Json, Router,
};
use std::{convert::Infallible, sync::Arc};
use tokio::sync::broadcast;
use typeless_core::engine::Engine;
use typeless_storage::Db;
use serde::Serialize;
use tower_http::cors::CorsLayer;

const HTML: &str = include_str!("web_ui.html");

#[derive(Clone)]
pub struct WebState {
    pub engine: Arc<Engine>,
    pub db:     Arc<Db>,
    pub tx:     Arc<broadcast::Sender<String>>,
}

pub async fn serve(engine: Arc<Engine>, db: Arc<Db>, port: u16) -> anyhow::Result<()> {
    let (tx, _) = broadcast::channel::<String>(64);
    let tx = Arc::new(tx);

    // 把引擎事件转发到 broadcast channel
    let tx2 = tx.clone();
    let mut ev_rx = engine.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = ev_rx.recv().await {
            if let Ok(s) = serde_json::to_string(&ev) {
                let _ = tx2.send(s);
            }
        }
    });

    let state = WebState { engine, db, tx };

    let app = Router::new()
        .route("/",              get(index))
        .route("/events",        get(sse_events))
        .route("/api/status",    get(api_status))
        .route("/api/toggle",    post(api_toggle))
        .route("/api/start",     post(api_start))
        .route("/api/stop",      post(api_stop))
        .route("/api/history",   get(api_history))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("🌐  Web UI 已启动 → http://localhost:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> { Html(HTML) }

async fn sse_events(State(s): State<WebState>)
    -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>>
{
    let rx = s.tx.subscribe();
    let stream = async_stream::stream! {
        let mut rx = rx;
        loop {
            match rx.recv().await {
                Ok(msg) => yield Ok(Event::default().data(msg)),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[derive(Serialize)]
struct StatusResp { status: String }

async fn api_status(State(s): State<WebState>) -> Json<StatusResp> {
    let status = format!("{:?}", s.engine.status()).to_lowercase();
    Json(StatusResp { status })
}

async fn api_toggle(State(s): State<WebState>) -> Json<serde_json::Value> {
    if s.engine.is_recording() {
        match s.engine.stop_and_process().await {
            Ok(t)  => Json(serde_json::json!({"ok":true,"text":t})),
            Err(e) => Json(serde_json::json!({"ok":false,"error":e.to_string()})),
        }
    } else {
        match s.engine.start_recording() {
            Ok(_)  => Json(serde_json::json!({"ok":true,"state":"recording"})),
            Err(e) => Json(serde_json::json!({"ok":false,"error":e.to_string()})),
        }
    }
}

async fn api_start(State(s): State<WebState>) -> Json<serde_json::Value> {
    match s.engine.start_recording() {
        Ok(_)  => Json(serde_json::json!({"ok":true})),
        Err(e) => Json(serde_json::json!({"ok":false,"error":e.to_string()})),
    }
}

async fn api_stop(State(s): State<WebState>) -> Json<serde_json::Value> {
    match s.engine.stop_and_process().await {
        Ok(t)  => Json(serde_json::json!({"ok":true,"text":t})),
        Err(e) => Json(serde_json::json!({"ok":false,"error":e.to_string()})),
    }
}

#[derive(Serialize)]
struct HistItem { created_at: String, raw_text: String, final_text: String, mode: String }

async fn api_history(State(s): State<WebState>) -> Json<Vec<HistItem>> {
    let list = s.db.list_history(20).unwrap_or_default();
    Json(list.into_iter().map(|h| HistItem {
        created_at: h.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        raw_text:   h.raw_text,
        final_text: h.final_text,
        mode:       h.mode,
    }).collect())
}
