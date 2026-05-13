use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Idle,
    Recording,
    Transcribing,
    Refining,
    Injecting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StatusEvent {
    Status { status: Status },
    PartialText { text: String },
    FinalText { text: String },
    Error { message: String },
    Info { message: String },
}
