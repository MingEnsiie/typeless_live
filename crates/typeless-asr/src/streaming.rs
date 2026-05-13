//! 流式 ASR：滑动窗口分段转写。
//! 简单策略：每收到 ~1.5s 新音频后，对累积窗口（最多 N 秒）调用 transcribe，
//! 输出当前最佳解读；UI 据此显示 partial。
use crate::{AsrEngine, AsrOptions};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct StreamingAsr {
    pub engine: Arc<dyn AsrEngine>,
    pub window_secs: usize,
    pub step_samples: usize,
}

impl StreamingAsr {
    pub fn new(engine: Arc<dyn AsrEngine>) -> Self {
        Self {
            engine,
            window_secs: 30,
            step_samples: 16_000 * 3 / 2, // 1.5s step
        }
    }

    /// 启动一个流式任务。`pcm_rx` 持续接收 PCM 块；返回部分转写结果。
    pub fn spawn(
        self: Arc<Self>,
        mut pcm_rx: mpsc::Receiver<Vec<i16>>,
        opts: AsrOptions,
    ) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel::<String>(8);
        tokio::spawn(async move {
            let mut buf: Vec<i16> = Vec::new();
            let mut last_emitted = String::new();
            let mut since_step = 0usize;
            while let Some(chunk) = pcm_rx.recv().await {
                buf.extend_from_slice(&chunk);
                since_step += chunk.len();
                if since_step >= self.step_samples && buf.len() >= 16_000 {
                    since_step = 0;
                    let max = self.window_secs * 16_000;
                    let start = buf.len().saturating_sub(max);
                    let slice = buf[start..].to_vec();
                    if let Ok(r) = self.engine.transcribe(&slice, &opts).await {
                        if r.text != last_emitted && !r.text.is_empty() {
                            last_emitted = r.text.clone();
                            let _ = tx.send(r.text).await;
                        }
                    }
                }
            }
            // 最终一遍
            if !buf.is_empty() {
                if let Ok(r) = self.engine.transcribe(&buf, &opts).await {
                    let _ = tx.send(r.text).await;
                }
            }
        });
        rx
    }
}
