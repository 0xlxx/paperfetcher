//! 统一 API 响应包装类型
//!
//! 每个 CLI 子命令对应一个 Response 类型，
//! 通过 CliOutput 包装后输出为标准 JSON 格式。

use serde::{Deserialize, Serialize};

use super::paper::{FetchResult, FetchSummary, LocalPaperEntry, Paper, PaperStatus};

// ── 搜索响应 ────────────────────────────────────────────

/// search 命令的返回结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// 数据来源标识（如 "openalex"）
    pub source: String,
    /// 原始查询字符串
    pub query: String,
    /// 数据源中的匹配总数
    pub total_results: u64,
    /// 本次返回的结果数量
    pub returned: u32,
    /// 论文列表
    pub results: Vec<Paper>,
}

// ── 查询响应 ────────────────────────────────────────────

/// lookup 命令的返回结果（单篇论文查询）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResponse {
    /// 数据来源标识
    pub source: String,
    /// 论文详情
    pub paper: Paper,
}

// ── 下载响应 ────────────────────────────────────────────

/// fetch 命令的返回结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    /// 每篇论文的下载结果
    pub results: Vec<FetchResult>,
    /// 批量操作汇总
    pub summary: FetchSummary,
}

// ── 列表响应 ────────────────────────────────────────────

/// list 命令的返回结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    /// 本地论文总数
    pub total: u32,
    /// 论文条目列表
    pub papers: Vec<LocalPaperEntry>,
}

// ── 状态响应 ────────────────────────────────────────────

/// status 命令的返回结果，直接复用 PaperStatus
#[allow(dead_code)]
pub type StatusResponse = PaperStatus;

// ── 删除响应 ────────────────────────────────────────────

/// remove 命令的返回结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveResponse {
    /// 被删除论文的 DOI
    pub doi: String,
    /// 是否删除了 PDF 文件
    pub removed_pdf: bool,
    /// 是否删除了元数据文件
    pub removed_metadata: bool,
    /// 本次操作是否产生了实际变更
    pub changed: bool,
}
