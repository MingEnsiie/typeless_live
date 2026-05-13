//! typeless-audio: 麦克风录制 + VAD。
//! 输出 16kHz / mono / i16 PCM。
pub mod capture;
pub mod vad;
pub mod wav;

pub use capture::{AudioCapture, CaptureHandle, CaptureMode};
pub use vad::SilenceDetector;

pub const SAMPLE_RATE: u32 = 16_000;
pub const CHANNELS: u16 = 1;
