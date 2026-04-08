use crate::config::settings::{ENV_CACHE_DIR, ENV_CONFIG_DIR, ENV_DATA_DIR, ENV_STATE_DIR};
use crate::errors::Result;
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePathSource {
    Default,
    Override,
}

impl RuntimePathSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Override => "override",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub db_path: PathBuf,
    pub master_key_path: PathBuf,
    pub config_source: RuntimePathSource,
    pub data_source: RuntimePathSource,
    pub state_source: RuntimePathSource,
    pub cache_source: RuntimePathSource,
}

impl AppPaths {
    pub fn detect() -> Result<Self> {
        let project_dirs =
            ProjectDirs::from("dev", "telegram-cli", "telegram-cli").ok_or_else(|| {
                crate::errors::TelegramCliError::Message(
                    "failed to resolve project directories".into(),
                )
            })?;

        let (config_dir, config_source) =
            resolve_runtime_dir(ENV_CONFIG_DIR, project_dirs.config_dir().to_path_buf());
        let (data_dir, data_source) =
            resolve_runtime_dir(ENV_DATA_DIR, project_dirs.data_dir().to_path_buf());
        let (state_dir, state_source) = resolve_runtime_dir(
            ENV_STATE_DIR,
            project_dirs
                .state_dir()
                .map(|path| path.to_path_buf())
                .unwrap_or_else(|| data_dir.join("state")),
        );
        let (cache_dir, cache_source) =
            resolve_runtime_dir(ENV_CACHE_DIR, project_dirs.cache_dir().to_path_buf());

        std::fs::create_dir_all(&config_dir)?;
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&state_dir)?;
        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            db_path: data_dir.join("state.sqlite"),
            master_key_path: config_dir.join("master.key"),
            config_dir,
            data_dir,
            state_dir,
            cache_dir,
            config_source,
            data_source,
            state_source,
            cache_source,
        })
    }
}

fn resolve_runtime_dir(env_var: &str, default_path: PathBuf) -> (PathBuf, RuntimePathSource) {
    match std::env::var_os(env_var) {
        Some(value) => (PathBuf::from(value), RuntimePathSource::Override),
        None => (default_path, RuntimePathSource::Default),
    }
}
