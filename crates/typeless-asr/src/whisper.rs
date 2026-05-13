#![cfg(feature = "whisper")]
use crate::{AsrEngine, AsrOptions, AsrResult};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperAsr {
    ctx: Arc<Mutex<WhisperContext>>,
    model_path: PathBuf,
}

impl WhisperAsr {
    pub fn load(model_path: PathBuf) -> Result<Self> {
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().unwrap(),
            WhisperContextParameters::default(),
        )?;
        Ok(Self { ctx: Arc::new(Mutex::new(ctx)), model_path })
    }
}

#[async_trait]
impl AsrEngine for WhisperAsr {
    async fn transcribe(&self, pcm: &[i16], opts: &AsrOptions) -> Result<AsrResult> {
        let pcm_f32: Vec<f32> = pcm.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
        let lang = opts.language.clone();
        let translate = opts.translate;
        let ctx = self.ctx.clone();

        let start = std::time::Instant::now();
        let text = tokio::task::spawn_blocking(move || -> Result<(String, Option<String>)> {
            let ctx = ctx.blocking_lock();
            let mut state = ctx.create_state()?;
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_translate(translate);
            params.set_print_progress(false);
            params.set_print_special(false);
            params.set_print_timestamps(false);
            params.set_print_realtime(false);
            params.set_n_threads(num_cpus());
            if let Some(l) = lang.as_deref() {
                if l != "auto" { params.set_language(Some(l)); }
            }
            state.full(params, &pcm_f32)?;
            let n = state.full_n_segments()?;
            let mut buf = String::new();
            for i in 0..n {
                buf.push_str(&state.full_get_segment_text(i)?);
            }
            Ok((buf, lang))
        }).await??;
        Ok(AsrResult {
            text: text.0.trim().to_string(),
            language: text.1,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
    fn name(&self) -> &str { "whisper" }
}

fn num_cpus() -> i32 {
    std::thread::available_parallelism().map(|n| n.get() as i32).unwrap_or(4)
}
