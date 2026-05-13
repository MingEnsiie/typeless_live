//! typeless-storage: 配置 / SQLite / keyring 统一封装。
pub mod config;
pub mod db;
pub mod paths;
pub mod secrets;

pub use config::{Settings, AsrSettings, LlmSettings, HotkeySettings, UiSettings, PrivacySettings};
pub use db::{Db, HistoryRecord, DictEntry};
pub use paths::AppPaths;
