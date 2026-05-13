use webrtc_vad::{SampleRate, Vad, VadMode};

/// 静音检测器：累计连续静音时间，超过阈值返回 true。
pub struct SilenceDetector {
    vad: Vad,
    sample_rate: usize,
    silence_ms: u32,
    threshold_ms: u32,
}

impl SilenceDetector {
    pub fn new(threshold_ms: u32) -> Self {
        let mut vad = Vad::new();
        vad.set_mode(VadMode::Aggressive);
        vad.set_sample_rate(SampleRate::Rate16kHz);
        Self {
            vad,
            sample_rate: 16000,
            silence_ms: 0,
            threshold_ms,
        }
    }

    /// 喂一帧 PCM；返回 (is_voice, exceed_silence)
    pub fn feed(&mut self, frame: &[i16]) -> (bool, bool) {
        // webrtc-vad 仅接受 10/20/30 ms 帧
        let frame_ms = (frame.len() * 1000 / self.sample_rate) as u32;
        let voice = self.vad.is_voice_segment(frame).unwrap_or(false);
        if voice {
            self.silence_ms = 0;
        } else {
            self.silence_ms = self.silence_ms.saturating_add(frame_ms);
        }
        (voice, self.silence_ms >= self.threshold_ms)
    }

    pub fn reset(&mut self) { self.silence_ms = 0; }
}
