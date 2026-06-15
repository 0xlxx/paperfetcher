//! 论文数据模型 — 核心领域对象
//!
//! 所有结构体均 derive Serialize/Deserialize，确保可以直接序列化为 JSON 输出。

use serde::{Deserialize, Serialize};

// ── 论文主结构 ──────────────────────────────────────────

/// 论文核心结构，包含元数据和开放获取信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    /// 数字对象标识符（如 10.1038/nature12373）
    pub doi: String,
    /// 论文标题
    pub title: String,
    /// 作者列表
    pub authors: Vec<Author>,
    /// 发表年份
    pub year: Option<u16>,
    /// 发表期刊或会议名称
    pub venue: Option<String>,
    /// 出版商名称
    pub publisher: Option<String>,
    /// 摘要文本（JSON 字段名映射为 "abstract"，避免与 Rust 关键字冲突）
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    /// 被引次数
    pub cited_by_count: Option<u64>,
    /// 是否为开放获取
    pub is_open_access: bool,
    /// 所有开放获取位置
    pub oa_locations: Vec<OaLocation>,
    /// 最佳开放获取 URL（优先选择出版商版本）
    pub best_oa_url: Option<String>,
    /// 数据来源标识（如 "openalex", "crossref", "unpaywall"）
    pub source: String,
}

// ── 作者 ────────────────────────────────────────────────

/// 作者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    /// 作者姓名
    pub name: String,
    /// 所属机构
    pub affiliation: Option<String>,
}

// ── 开放获取位置 ────────────────────────────────────────

/// 开放获取位置信息，描述 PDF 的可获取来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OaLocation {
    /// 访问 URL
    pub url: String,
    /// 来源类型：publisher（出版商）或 repository（预印本仓库）
    pub host_type: Option<String>,
    /// 许可协议（如 cc-by, cc-by-nc）
    pub license: Option<String>,
    /// 版本标识（如 publishedVersion, acceptedVersion）
    pub version: Option<String>,
}

// ── 下载结果 ────────────────────────────────────────────

/// 单篇论文的下载操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    /// 论文 DOI
    pub doi: String,
    /// 本次操作是否产生了文件变更
    pub changed: bool,
    /// 执行的操作类型
    pub action: FetchAction,
    /// 下载后的本地 PDF 路径
    pub path: Option<String>,
    /// 元数据 JSON 文件路径
    pub metadata_path: Option<String>,
    /// 文件大小（字节）
    pub size_bytes: Option<u64>,
    /// 下载来源名称
    pub source: Option<String>,
    /// 跳过或失败的原因说明
    pub reason: Option<String>,
}

/// 下载操作的三种结果类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FetchAction {
    /// 成功下载
    Downloaded,
    /// 跳过（已存在或无需更新）
    Skipped,
    /// 下载失败
    Failed,
}

// ── 批量下载统计 ────────────────────────────────────────

/// 批量下载的汇总统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchSummary {
    /// 总处理数量
    pub total: u32,
    /// 成功下载数量
    pub downloaded: u32,
    /// 跳过数量
    pub skipped: u32,
    /// 失败数量
    pub failed: u32,
}

// ── 本地论文状态 ────────────────────────────────────────

/// 论文在本地存储中的状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperStatus {
    /// 论文 DOI
    pub doi: String,
    /// 是否已有 PDF 文件
    pub has_pdf: bool,
    /// PDF 文件路径
    pub pdf_path: Option<String>,
    /// PDF 文件大小（字节）
    pub pdf_size_bytes: Option<u64>,
    /// 是否已有元数据文件
    pub has_metadata: bool,
    /// 元数据文件路径
    pub metadata_path: Option<String>,
    /// 下载时间（ISO 8601）
    pub downloaded_at: Option<String>,
}

// ── 本地索引条目 ────────────────────────────────────────

/// 本地论文索引条目，用于 list 命令的输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPaperEntry {
    /// 论文 DOI
    pub doi: String,
    /// 论文标题
    pub title: String,
    /// 作者名字列表（简化表示，仅保留姓名字符串）
    pub authors: Vec<String>,
    /// 发表年份
    pub year: Option<u16>,
    /// 本地 PDF 文件路径
    pub pdf_path: String,
    /// 元数据文件路径
    pub metadata_path: Option<String>,
    /// 下载时间（ISO 8601 格式）
    pub downloaded_at: String,
    /// 文件大小（字节）
    pub size_bytes: u64,
}
