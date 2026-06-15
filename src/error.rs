//! 项目级错误类型与统一输出格式
//!
//! 所有 CLI 命令的输出遵循统一的 JSON 结构：
//! - 成功: {"status":"success", ...data}
//! - 失败: {"status":"error", "error":{"code":"...","message":"...","suggestions":[...]}}

use serde::{Deserialize, Serialize};

// ── 退出码常量 ──────────────────────────────────────────
/// 程序退出码，遵循 sysexits.h 风格约定
pub mod exit_code {
    #[allow(dead_code)]
    pub const SUCCESS: i32 = 0;
    pub const GENERAL: i32 = 1;
    pub const ARGS: i32 = 2;
    pub const API: i32 = 3;
    pub const NOT_FOUND: i32 = 4;
    pub const IO: i32 = 5;
}

// ── 核心错误类型 ────────────────────────────────────────
/// 应用层统一错误类型，涵盖所有可能的失败场景
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// API 调用返回非成功状态码
    #[error("API error (HTTP {status_code}): {body}")]
    ApiError {
        #[source]
        source: reqwest::Error,
        /// HTTP 状态码
        status_code: u16,
        /// 响应体原文（用于调试）
        body: String,
    },

    /// 网络不可达（DNS 失败、连接超时等）
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// 触发了 API 限流策略
    #[allow(dead_code)]
    #[error("Rate limited, retry after {retry_after_secs}s")]
    RateLimited {
        #[source]
        source: reqwest::Error,
        /// 建议重试等待时长（秒）
        retry_after_secs: u64,
    },

    /// 指定的 DOI 在数据源中未找到
    #[error("DOI not found: {doi}")]
    NotFound { doi: String },

    /// 文件 IO 操作失败
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// 配置文件读取/解析错误
    #[error("Config error: {0}")]
    ConfigError(String),

    /// 数据解析失败（JSON / TOML 等）
    #[error("Parse error: {0}")]
    ParseError(String),

    /// DOI 格式不符合规范（必须以 10. 开头）
    #[error("Invalid DOI format: {doi}")]
    InvalidDoi { doi: String },
}

impl AppError {
    /// 将错误转换为对应的进程退出码
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::ApiError { .. } => exit_code::API,
            Self::NetworkError(_) => exit_code::API,
            Self::RateLimited { .. } => exit_code::API,
            Self::NotFound { .. } => exit_code::NOT_FOUND,
            Self::IoError(_) => exit_code::IO,
            Self::ConfigError(_) => exit_code::GENERAL,
            Self::ParseError(_) => exit_code::GENERAL,
            Self::InvalidDoi { .. } => exit_code::ARGS,
        }
    }

    /// 将错误转换为结构化的 ErrorResponse，供 JSON 输出使用
    pub fn to_error_response(&self) -> ErrorResponse {
        match self {
            Self::ApiError {
                status_code, body, ..
            } => ErrorResponse {
                code: "api_error".into(),
                message: format!("API returned HTTP {status_code}: {body}"),
                suggestions: vec![
                    "检查 API 密钥是否有效".into(),
                    "稍后重试请求".into(),
                ],
            },
            Self::NetworkError(e) => ErrorResponse {
                code: "network_error".into(),
                message: format!("网络请求失败: {e}"),
                suggestions: vec![
                    "检查网络连接".into(),
                    "确认目标服务是否可访问".into(),
                ],
            },
            Self::RateLimited {
                retry_after_secs, ..
            } => ErrorResponse {
                code: "rate_limited".into(),
                message: format!("请求被限流，建议 {retry_after_secs} 秒后重试"),
                suggestions: vec![
                    format!("等待 {retry_after_secs} 秒后重试"),
                    "减少请求频率".into(),
                ],
            },
            Self::NotFound { doi } => ErrorResponse {
                code: "not_found".into(),
                message: format!("DOI 未找到: {doi}"),
                suggestions: vec![
                    "检查 DOI 是否拼写正确".into(),
                    "尝试使用其他数据源查询".into(),
                ],
            },
            Self::IoError(e) => ErrorResponse {
                code: "io_error".into(),
                message: format!("文件操作失败: {e}"),
                suggestions: vec![
                    "检查文件路径和权限".into(),
                    "确认磁盘空间充足".into(),
                ],
            },
            Self::ConfigError(msg) => ErrorResponse {
                code: "config_error".into(),
                message: msg.clone(),
                suggestions: vec![
                    "检查配置文件格式".into(),
                    "运行 paperfetcher config --show 查看当前配置".into(),
                ],
            },
            Self::ParseError(msg) => ErrorResponse {
                code: "parse_error".into(),
                message: msg.clone(),
                suggestions: vec!["检查输入数据格式".into()],
            },
            Self::InvalidDoi { doi } => ErrorResponse {
                code: "invalid_doi".into(),
                message: format!("DOI 格式无效: {doi}"),
                suggestions: vec![
                    "DOI 必须以 '10.' 开头".into(),
                    "示例: 10.1000/xyz123".into(),
                ],
            },
        }
    }
}

// ── 结构化错误响应 ──────────────────────────────────────

/// 错误的结构化表示，包含机器可读的 code 和人类可读的 message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// 机器可读的错误代码（如 "not_found", "api_error"）
    pub code: String,
    /// 人类可读的错误描述
    pub message: String,
    /// 给 LLM/用户的修复建议列表
    pub suggestions: Vec<String>,
}

// ── 统一输出包装 ────────────────────────────────────────

/// CLI 统一输出包装——成功时携带数据，失败时携带结构化错误
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum CliOutput<T: Serialize> {
    /// 成功响应：data 会被 flatten 到顶层 JSON 对象中
    #[serde(rename = "success")]
    Success {
        #[serde(flatten)]
        data: T,
    },
    /// 错误响应：包含结构化错误信息
    #[serde(rename = "error")]
    Error { error: ErrorResponse },
}

impl<T: Serialize> CliOutput<T> {
    /// 构造成功输出
    pub fn success(data: T) -> Self {
        Self::Success { data }
    }
}

impl CliOutput<()> {
    /// 从 AppError 构造错误输出（无需指定泛型参数）
    pub fn from_error(err: &AppError) -> CliOutput<serde_json::Value> {
        CliOutput::Error {
            error: err.to_error_response(),
        }
    }
}

// ── serde_json::Error → AppError 转换 ──────────────────
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::ParseError(e.to_string())
    }
}

// ── toml::de::Error → AppError 转换 ────────────────────
impl From<toml::de::Error> for AppError {
    fn from(e: toml::de::Error) -> Self {
        Self::ConfigError(e.to_string())
    }
}
