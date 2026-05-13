use crate::state::{Status, StatusEvent};
use chrono::Utc;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use typeless_asr::AsrEngine;
use typeless_audio::{AudioCapture, CaptureHandle, CaptureMode};
use typeless_inject::TextInjector;
use typeless_llm::PostProcessor;
use typeless_storage::{Db, HistoryRecord};
use uuid::Uuid;

#[derive(Clone)]
pub struct EngineConfig {
    pub mode: CaptureMode,
    pub language: Option<String>,
    pub translate: bool,
    pub save_history: bool,
    pub auto_inject: bool,
}
impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            mode: CaptureMode::Toggle,
            language: None,
            translate: false,
            save_history: true,
            auto_inject: true,
        }
    }
}

pub struct Engine {
    pub asr: Arc<dyn AsrEngine>,
    pub post: Arc<PostProcessor>,
    pub injector: Arc<dyn TextInjector>,
    pub db: Option<Arc<Db>>,
    pub cfg: EngineConfig,
    status_tx: broadcast::Sender<StatusEvent>,
    capture: Mutex<Option<CaptureHandle>>,
    current_status: Mutex<Status>,
}

impl Engine {
    pub fn new(
        asr: Arc<dyn AsrEngine>,
        post: Arc<PostProcessor>,
        injector: Arc<dyn TextInjector>,
        db: Option<Arc<Db>>,
        cfg: EngineConfig,
    ) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            asr, post, injector, db, cfg,
            status_tx: tx,
            capture: Mutex::new(None),
            current_status: Mutex::new(Status::Idle),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<StatusEvent> {
        self.status_tx.subscribe()
    }

    pub fn status(&self) -> Status { *self.current_status.lock() }

    fn set_status(&self, s: Status) {
        *self.current_status.lock() = s;
        let _ = self.status_tx.send(StatusEvent::Status { status: s });
    }

    fn emit(&self, e: StatusEvent) { let _ = self.status_tx.send(e); }

    /// 开始录音。若已在录音则忽略。
    pub fn start_recording(&self) -> anyhow::Result<()> {
        let mut cap = self.capture.lock();
        if cap.is_some() { return Ok(()); }
        let h = AudioCapture::start(self.cfg.mode)?;
        *cap = Some(h);
        self.set_status(Status::Recording);
        info!("recording started");
        Ok(())
    }

    /// 结束录音并执行完整 pipeline。
    pub async fn stop_and_process(self: &Arc<Self>) -> anyhow::Result<Option<String>> {
        let handle = {
            let mut g = self.capture.lock();
            g.take()
        };
        let Some(handle) = handle else {
            return Ok(None);
        };
        handle.stop();
        // 给录音线程时间 flush
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        let pcm = handle.collected();
        info!(samples = pcm.len(), "recording stopped");

        if pcm.len() < 16_000 / 5 {
            // <0.2s
            self.emit(StatusEvent::Info { message: "录音太短，已忽略".into() });
            self.set_status(Status::Idle);
            return Ok(None);
        }

        // ASR
        self.set_status(Status::Transcribing);
        let asr_opts = typeless_asr::AsrOptions {
            language: self.cfg.language.clone(),
            translate: self.cfg.translate,
        };
        let asr_res = match self.asr.transcribe(&pcm, &asr_opts).await {
            Ok(r) => r,
            Err(e) => {
                error!(error=%e, "asr failed");
                self.emit(StatusEvent::Error { message: format!("ASR 失败: {e}") });
                self.set_status(Status::Error);
                return Err(e);
            }
        };
        info!(text=%asr_res.text, "asr done");
        self.emit(StatusEvent::PartialText { text: asr_res.text.clone() });

        if asr_res.text.trim().is_empty() {
            self.emit(StatusEvent::Info { message: "未识别到内容".into() });
            self.set_status(Status::Idle);
            return Ok(None);
        }

        // LLM
        self.set_status(Status::Refining);
        let final_text = match self.post.refine(&asr_res.text).await {
            Ok(t) => t,
            Err(e) => {
                warn!(error=%e, "llm failed, fallback to raw");
                self.emit(StatusEvent::Info { message: format!("LLM 失败，使用原文: {e}") });
                asr_res.text.clone()
            }
        };
        self.emit(StatusEvent::FinalText { text: final_text.clone() });

        // Inject
        if self.cfg.auto_inject && !final_text.is_empty() {
            self.set_status(Status::Injecting);
            if let Err(e) = self.injector.inject(&final_text).await {
                error!(error=%e, "inject failed");
                self.emit(StatusEvent::Error { message: format!("注入失败: {e}") });
            }
        }

        // History
        if self.cfg.save_history {
            if let Some(db) = &self.db {
                let app_ctx = typeless_context::AppContext::detect();
                let rec = HistoryRecord {
                    id: Uuid::new_v4().to_string(),
                    created_at: Utc::now(),
                    raw_text: asr_res.text,
                    final_text: final_text.clone(),
                    app: app_ctx.summary(),
                    mode: self.post.mode.as_str().to_string(),
                    asr_model: self.asr.name().to_string(),
                    llm_model: self.post.opts.model.clone(),
                    duration_ms: asr_res.duration_ms as i64,
                    starred: false,
                };
                if let Err(e) = db.insert_history(&rec) {
                    warn!(error=%e, "history insert failed");
                }
            }
        }

        self.set_status(Status::Idle);
        Ok(Some(final_text))
    }

    pub fn is_recording(&self) -> bool { self.capture.lock().is_some() }
}
