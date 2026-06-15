//! 输出格式化模块
//!
//! 负责将命令执行结果以统一格式输出到 stdout/stderr。
//! - JSON 模式：输出 {"status":"success", ...data} 结构（给 LLM Agent 消费）
//! - JSONL 模式：同 JSON，但保证单行输出（适合流式处理）
//! - Text 模式：美化输出（给人类阅读）

use serde::Serialize;

use crate::error::{AppError, CliOutput};

/// 输出格式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// 标准 JSON，缩进美化
    #[default]
    Json,
    /// 纯文本，人类可读
    Text,
    /// JSON Lines，单行紧凑 JSON（适合管道和流式处理）
    Jsonl,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Text => write!(f, "text"),
            Self::Jsonl => write!(f, "jsonl"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "text" => Ok(Self::Text),
            "jsonl" => Ok(Self::Jsonl),
            other => Err(format!("未知输出格式: '{other}'，支持: json, text, jsonl")),
        }
    }
}

/// 打印成功输出到 stdout
///
/// - JSON 模式：包装为 {"status":"success", ...data} 并美化缩进
/// - JSONL 模式：同 JSON 但紧凑单行
/// - Text 模式：使用 serde_json 美化输出（后续可按类型定制 Display）
pub fn print_success<T: Serialize>(data: &T, format: OutputFormat) {
    let output = CliOutput::success(data);
    match format {
        OutputFormat::Json => {
            // 美化 JSON 输出，方便人类阅读和调试
            let json = serde_json::to_string_pretty(&output)
                .expect("序列化成功输出不应失败");
            println!("{json}");
        }
        OutputFormat::Jsonl => {
            // 紧凑单行 JSON，适合管道处理
            let json =
                serde_json::to_string(&output).expect("序列化成功输出不应失败");
            println!("{json}");
        }
        OutputFormat::Text => {
            // 文本模式：将 data 美化为 JSON（后续可改为自定义 Display）
            let json = serde_json::to_string_pretty(data)
                .expect("序列化成功输出不应失败");
            println!("{json}");
        }
    }
}

/// 打印结构化错误到 stdout（JSON）并同步输出到 stderr（文本）
///
/// 确保 LLM Agent 可以解析 stdout 的 JSON 错误，
/// 同时人类用户也能在 stderr 看到可读的错误信息。
pub fn print_error(err: &AppError) {
    // stdout: 结构化 JSON 错误（给 LLM Agent）
    let output = CliOutput::<()>::from_error(err);
    let json =
        serde_json::to_string_pretty(&output).expect("序列化错误输出不应失败");
    println!("{json}");

    // stderr: 人类可读的错误信息
    eprintln!("error: {err}");
}

/// 信息日志输出到 stderr（给人看的进度/状态信息）
///
/// 不影响 stdout 的 JSON 输出流，确保管道处理安全。
pub fn eprintln_info(msg: &str) {
    eprintln!("[info] {msg}");
}
