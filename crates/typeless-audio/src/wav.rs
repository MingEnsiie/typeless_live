use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::Path;

pub fn save_pcm_i16<P: AsRef<Path>>(pcm: &[i16], sample_rate: u32, path: P) -> Result<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut w = WavWriter::create(path, spec)?;
    for s in pcm { w.write_sample(*s)?; }
    w.finalize()?;
    Ok(())
}

pub fn pcm_to_f32(pcm: &[i16]) -> Vec<f32> {
    pcm.iter().map(|&s| s as f32 / i16::MAX as f32).collect()
}
