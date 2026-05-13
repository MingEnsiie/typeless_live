use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
    pub models_dir: PathBuf,
    pub config_file: PathBuf,
    pub db_file: PathBuf,
}

impl AppPaths {
    pub fn discover() -> anyhow::Result<Self> {
        let pd = ProjectDirs::from("app", "typeless", "typeless")
            .ok_or_else(|| anyhow::anyhow!("cannot determine project directories"))?;
        let config_dir = pd.config_dir().to_path_buf();
        let data_dir = pd.data_dir().to_path_buf();
        let cache_dir = pd.cache_dir().to_path_buf();
        let log_dir = data_dir.join("logs");
        let models_dir = data_dir.join("models");
        let config_file = config_dir.join("config.toml");
        let db_file = data_dir.join("typeless.db");
        for p in [&config_dir, &data_dir, &cache_dir, &log_dir, &models_dir] {
            std::fs::create_dir_all(p).ok();
        }
        Ok(Self {
            config_dir, data_dir, cache_dir, log_dir, models_dir, config_file, db_file
        })
    }
}
