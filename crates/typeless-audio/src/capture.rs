use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{CHANNELS, SAMPLE_RATE};

#[derive(Debug, Clone, Copy)]
pub enum CaptureMode {
    /// Push-to-talk: 显式 stop()
    PushToTalk,
    /// Toggle: 显式 stop()
    Toggle,
}

pub struct AudioCapture;

pub struct CaptureHandle {
    stop: Arc<Mutex<bool>>,
    pub rx: mpsc::Receiver<Vec<i16>>,
    /// 累积的全量 PCM（i16）
    pub all: Arc<Mutex<Vec<i16>>>,
    _stream_thread: std::thread::JoinHandle<()>,
}

impl CaptureHandle {
    pub fn stop(&self) {
        *self.stop.lock() = true;
    }
    pub fn collected(&self) -> Vec<i16> {
        self.all.lock().clone()
    }
}

impl AudioCapture {
    /// 启动录音；返回 handle。在调用 stop() 后流终止。
    pub fn start(_mode: CaptureMode) -> Result<CaptureHandle> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("no default input device"))?;
        let supported = device
            .default_input_config()
            .map_err(|e| anyhow::anyhow!("default config: {e}"))?;
        info!(?supported, "input device");

        let in_sr = supported.sample_rate().0;
        let in_ch = supported.channels();
        let sample_format = supported.sample_format();
        let cfg: StreamConfig = supported.into();

        let (tx, rx) = mpsc::channel::<Vec<i16>>(64);
        let stop = Arc::new(Mutex::new(false));
        let all: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));

        let stop2 = stop.clone();
        let all2 = all.clone();
        let tx2 = tx.clone();

        // cpal Stream 不 Send => 在独立线程中运行
        let handle = std::thread::spawn(move || {
            let err_fn = |e| warn!(error=%e, "stream error");
            let resampler = SimpleResampler::new(in_sr, SAMPLE_RATE, in_ch);
            let process = move |pcm_i16: Vec<i16>| {
                let mono16k = resampler.process(&pcm_i16);
                if !mono16k.is_empty() {
                    all2.lock().extend_from_slice(&mono16k);
                    let _ = tx2.try_send(mono16k);
                }
            };
            let process = Arc::new(Mutex::new(process));

            let stream = match sample_format {
                SampleFormat::F32 => {
                    let p = process.clone();
                    device.build_input_stream(
                        &cfg,
                        move |data: &[f32], _| {
                            let pcm: Vec<i16> = data.iter()
                                .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                                .collect();
                            (p.lock())(pcm);
                        },
                        err_fn, None,
                    )
                }
                SampleFormat::I16 => {
                    let p = process.clone();
                    device.build_input_stream(
                        &cfg,
                        move |data: &[i16], _| { (p.lock())(data.to_vec()); },
                        err_fn, None,
                    )
                }
                SampleFormat::U16 => {
                    let p = process.clone();
                    device.build_input_stream(
                        &cfg,
                        move |data: &[u16], _| {
                            let pcm: Vec<i16> = data.iter()
                                .map(|&s| (s as i32 - i16::MAX as i32) as i16).collect();
                            (p.lock())(pcm);
                        },
                        err_fn, None,
                    )
                }
                _ => {
                    warn!("unsupported sample format");
                    return;
                }
            };
            let stream = match stream {
                Ok(s) => s,
                Err(e) => { warn!(error=%e, "build stream"); return; }
            };
            if let Err(e) = stream.play() {
                warn!(error=%e, "stream play");
                return;
            }
            // 等待 stop 信号
            loop {
                if *stop2.lock() { break; }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            drop(stream);
        });

        let _ = CHANNELS; // 避免未使用警告
        Ok(CaptureHandle {
            stop, rx, all, _stream_thread: handle,
        })
    }
}

/// 极简重采样：channel-mix + 整数比线性插值。
struct SimpleResampler {
    in_sr: u32,
    out_sr: u32,
    in_ch: u16,
}
impl SimpleResampler {
    fn new(in_sr: u32, out_sr: u32, in_ch: u16) -> Self { Self { in_sr, out_sr, in_ch } }
    fn process(&self, input: &[i16]) -> Vec<i16> {
        // 1. 多声道下混到单声道
        let mono: Vec<i16> = if self.in_ch <= 1 {
            input.to_vec()
        } else {
            input.chunks(self.in_ch as usize)
                .map(|frame| {
                    let sum: i32 = frame.iter().map(|&s| s as i32).sum();
                    (sum / frame.len() as i32) as i16
                })
                .collect()
        };
        if self.in_sr == self.out_sr {
            return mono;
        }
        // 2. 线性插值重采样
        let ratio = self.out_sr as f64 / self.in_sr as f64;
        let out_len = ((mono.len() as f64) * ratio) as usize;
        let mut out = Vec::with_capacity(out_len);
        for i in 0..out_len {
            let pos = i as f64 / ratio;
            let idx = pos.floor() as usize;
            let frac = pos - idx as f64;
            let s0 = *mono.get(idx).unwrap_or(&0) as f64;
            let s1 = *mono.get(idx + 1).unwrap_or(&0) as f64;
            out.push(((s0 * (1.0 - frac) + s1 * frac) as i16).clamp(i16::MIN, i16::MAX));
        }
        out
    }
}
