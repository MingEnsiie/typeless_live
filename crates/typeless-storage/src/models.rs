//! 模型注册与下载管理。
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub kind: String, // "whisper" | "llm"
    pub name: String,
    pub url: String,
    pub size_mb: u32,
    pub sha256: Option<String>,
    pub description: String,
}

pub fn registry() -> Vec<ModelDescriptor> {
    vec![
        ModelDescriptor {
            kind: "whisper".into(),
            name: "ggml-tiny.bin".into(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin".into(),
            size_mb: 75,
            sha256: None,
            description: "Whisper tiny - 多语种 75MB，CPU 实时".into(),
        },
        ModelDescriptor {
            kind: "whisper".into(),
            name: "ggml-base.bin".into(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin".into(),
            size_mb: 142,
            sha256: None,
            description: "Whisper base - 推荐，平衡速度与精度".into(),
        },
        ModelDescriptor {
            kind: "whisper".into(),
            name: "ggml-small.bin".into(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin".into(),
            size_mb: 466,
            sha256: None,
            description: "Whisper small - 高精度，需 4GB RAM".into(),
        },
        ModelDescriptor {
            kind: "whisper".into(),
            name: "ggml-medium.bin".into(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin".into(),
            size_mb: 1500,
            sha256: None,
            description: "Whisper medium - 顶级精度，需 GPU 推荐".into(),
        },
    ]
}

pub fn find(name: &str) -> Option<ModelDescriptor> {
    registry().into_iter().find(|m| m.name == name)
}
