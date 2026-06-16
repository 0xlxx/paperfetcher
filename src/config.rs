// 配置管理 — 多层配置加载（默认值 → 文件 → 环境变量 → CLI）
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::output::OutputFormat;
use crate::sources::SourceName;

/// 主配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 用户邮箱（必填，用于 API 礼貌池）
    pub email: String,
    /// 本地数据目录
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    /// 默认输出格式
    #[serde(default)]
    pub default_output: OutputFormat,
    /// 搜索配置
    #[serde(default)]
    pub search: SearchConfig,
    /// 下载配置
    #[serde(default)]
    pub fetch: FetchConfig,
    /// 数据源配置
    #[serde(default)]
    pub sources: SourcesConfig,
}

/// 搜索相关配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchConfig {
    /// 默认返回数量
    pub default_limit: u32,
    /// 默认数据源
    pub default_source: SourceName,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_limit: 10,
            default_source: SourceName::OpenAlex,
        }
    }
}

/// 下载相关配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FetchConfig {
    /// 最大并发下载数
    pub max_concurrent: usize,
    /// 下载超时（秒）
    pub timeout_secs: u64,
    /// 是否同时保存元数据
    pub with_metadata: bool,
    /// 是否覆盖已存在文件
    pub overwrite: bool,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            timeout_secs: 30,
            with_metadata: true,
            overwrite: false,
        }
    }
}

/// 数据源相关配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SourcesConfig {
    /// PDF 下载回退顺序
    pub pdf_sources: Vec<SourceName>,
    /// 搜索优先源
    pub search_source: SourceName,
}

impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            pdf_sources: vec![SourceName::OpenAlex, SourceName::Unpaywall],
            search_source: SourceName::OpenAlex,
        }
    }
}

// OutputFormat 的 serde 支持
impl<'de> Deserialize<'de> for OutputFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Config {
    /// 加载配置：默认值 → TOML 文件 → 环境变量 → CLI 参数
    ///
    /// 优先级从低到高：代码默认值 < config.toml < 环境变量 < CLI 参数
    pub fn load(
        cli_email: Option<&str>,
        cli_config_path: Option<&Path>,
        cli_data_dir: Option<&Path>,
    ) -> Result<Config, AppError> {
        // 第一步：确定配置文件路径
        let config_path = if let Some(p) = cli_config_path {
            p.to_path_buf()
        } else {
            default_config_path()
        };

        // 第二步：尝试从文件加载配置（不存在则跳过）
        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).map_err(|e| {
                AppError::ConfigError(format!(
                    "failed to read config file {}: {e}",
                    config_path.display()
                ))
            })?;
            toml::from_str::<Config>(&content).map_err(|e| {
                AppError::ConfigError(format!(
                    "failed to parse config file {}: {e}",
                    config_path.display()
                ))
            })?
        } else {
            // 使用全默认值
            Config {
                email: String::new(),
                data_dir: default_data_dir(),
                default_output: OutputFormat::default(),
                search: SearchConfig::default(),
                fetch: FetchConfig::default(),
                sources: SourcesConfig::default(),
            }
        };

        // 第三步：环境变量覆盖
        if let Ok(email) = std::env::var("PAPERFETCHER_EMAIL") {
            if !email.is_empty() {
                config.email = email;
            }
        }
        if let Ok(data_dir) = std::env::var("PAPERFETCHER_DATA_DIR") {
            if !data_dir.is_empty() {
                config.data_dir = PathBuf::from(data_dir);
            }
        }

        // 第四步：CLI 参数覆盖（最高优先级）
        if let Some(email) = cli_email {
            config.email = email.to_string();
        }
        if let Some(data_dir) = cli_data_dir {
            config.data_dir = data_dir.to_path_buf();
        }

        // 校验：email 必须有值
        if config.email.is_empty() {
            return Err(AppError::ConfigError(
                "email is required. Set via --email, PAPERFETCHER_EMAIL env var, or config file"
                    .to_string(),
            ));
        }

        Ok(config)
    }
}

/// 默认配置文件路径
pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("paperfetcher")
        .join("config.toml")
}

/// 默认数据目录
fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("paperfetcher")
}
