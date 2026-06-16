use std::path::PathBuf;
use crate::cli::ConfigAction;
use crate::config::{Config, default_config_path};
use crate::error::AppError;
use crate::output::OutputFormat;

#[derive(serde::Serialize)]
pub struct ConfigResponse {
    pub message: String,
    pub path: PathBuf,
    pub config: Config,
}

pub fn execute(action: ConfigAction, current_config: &Config) -> Result<ConfigResponse, AppError> {
    let path = default_config_path();

    match action {
        ConfigAction::Show => {
            Ok(ConfigResponse {
                message: "Current configuration".to_string(),
                path,
                config: current_config.clone(),
            })
        }
        ConfigAction::Set {
            email,
            output,
            limit,
            max_concurrent,
            timeout_secs,
        } => {
            let mut new_config = current_config.clone();
            let mut changed = false;

            if let Some(e) = email {
                new_config.email = e;
                changed = true;
            }
            if let Some(o) = output {
                new_config.default_output = OutputFormat::from(o);
                changed = true;
            }
            if let Some(l) = limit {
                new_config.search.default_limit = l;
                changed = true;
            }
            if let Some(m) = max_concurrent {
                new_config.fetch.max_concurrent = m;
                changed = true;
            }
            if let Some(t) = timeout_secs {
                new_config.fetch.timeout_secs = t;
                changed = true;
            }

            if changed {
                // Ensure directory exists
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        AppError::IoError(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to create config directory {}: {}", parent.display(), e),
                        ))
                    })?;
                }

                // Write configuration to file
                let content = toml::to_string(&new_config).map_err(|e| {
                    AppError::ConfigError(format!("Failed to serialize config: {}", e))
                })?;

                std::fs::write(&path, content).map_err(|e| {
                    AppError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to write config file {}: {}", path.display(), e),
                    ))
                })?;

                Ok(ConfigResponse {
                    message: "Configuration updated".to_string(),
                    path,
                    config: new_config,
                })
            } else {
                Ok(ConfigResponse {
                    message: "No configuration values changed".to_string(),
                    path,
                    config: new_config,
                })
            }
        }
    }
}
