// CLI 定义 — 使用 clap derive 定义命令行接口
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::output::OutputFormat;
use crate::sources::SourceName;

/// LLM-agent-friendly CLI for academic paper search and retrieval
#[derive(Parser, Debug)]
#[command(
    name = "paperfetcher",
    about = "LLM-agent-friendly CLI for academic paper search and retrieval",
    version,
    long_about = None
)]
pub struct Cli {
    /// 输出格式: json, text, jsonl
    #[arg(long, global = true, env = "PAPERFETCHER_OUTPUT", default_value = "json")]
    pub output: CliOutputFormat,

    /// 静默模式：抑制 stderr 信息日志
    #[arg(long, global = true)]
    pub quiet: bool,

    /// 详细模式：输出调试信息到 stderr
    #[arg(long, global = true)]
    pub verbose: bool,

    /// 用户邮箱（用于 API 礼貌池，某些 API 必需）
    #[arg(long, global = true, env = "PAPERFETCHER_EMAIL")]
    pub email: Option<String>,

    /// 自定义配置文件路径
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// 数据存储目录
    #[arg(long, global = true, env = "PAPERFETCHER_DATA_DIR")]
    pub data_dir: Option<PathBuf>,

    /// 子命令
    #[command(subcommand)]
    pub command: Commands,
}

/// clap 可解析的输出格式（包装 OutputFormat 以实现 ValueEnum）
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliOutputFormat {
    Json,
    Text,
    Jsonl,
}

impl From<CliOutputFormat> for OutputFormat {
    fn from(f: CliOutputFormat) -> Self {
        match f {
            CliOutputFormat::Json => OutputFormat::Json,
            CliOutputFormat::Text => OutputFormat::Text,
            CliOutputFormat::Jsonl => OutputFormat::Jsonl,
        }
    }
}

/// clap 可解析的数据源名称
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliSourceName {
    Unpaywall,
    SemanticScholar,
    OpenAlex,
    CrossRef,
}

impl From<CliSourceName> for SourceName {
    fn from(s: CliSourceName) -> Self {
        match s {
            CliSourceName::Unpaywall => SourceName::Unpaywall,
            CliSourceName::SemanticScholar => SourceName::SemanticScholar,
            CliSourceName::OpenAlex => SourceName::OpenAlex,
            CliSourceName::CrossRef => SourceName::CrossRef,
        }
    }
}

/// 所有子命令
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 搜索学术论文
    Search {
        /// 搜索关键词
        query: String,

        /// 数据源
        #[arg(long, short, default_value = "open-alex")]
        source: CliSourceName,

        /// 最大返回数量
        #[arg(long, short, default_value = "10")]
        limit: u32,

        /// 按年份过滤（如 2023, 2020-2024）
        #[arg(long, short)]
        year: Option<String>,

        /// 仅返回开放获取论文
        #[arg(long)]
        open_access: bool,

        /// 排序方式
        #[arg(long, default_value = "relevance")]
        sort: String,
    },

    /// 按 DOI 查询单篇论文详情
    Lookup {
        /// 论文 DOI
        doi: String,

        /// 数据源
        #[arg(long, short, default_value = "open-alex")]
        source: CliSourceName,
    },

    /// 下载论文 PDF
    Fetch {
        /// DOI 或包含 DOI 列表的文件路径
        #[arg(required_unless_present = "stdin")]
        doi_or_file: Option<String>,

        /// 输出目录
        #[arg(long, short)]
        output_dir: Option<PathBuf>,

        /// 文件名模板
        #[arg(long)]
        filename_template: Option<String>,

        /// 覆盖已存在的文件
        #[arg(long)]
        overwrite: bool,

        /// 同时保存论文元数据
        #[arg(long)]
        with_metadata: bool,

        /// 最大并发下载数
        #[arg(long, default_value = "3")]
        max_concurrent: usize,

        /// 下载超时（秒）
        #[arg(long, default_value = "30")]
        timeout: u64,

        /// 用于获取 PDF URL 的数据源（可指定多个作为回退）
        #[arg(long, short)]
        source: Vec<CliSourceName>,

        /// 从 stdin 读取 DOI 列表
        #[arg(long)]
        stdin: bool,
    },

    /// 列出本地已下载的论文
    List {
        /// 搜索过滤（标题/作者关键词）
        #[arg(long, short)]
        filter: Option<String>,

        /// 按年份过滤
        #[arg(long, short)]
        year: Option<String>,

        /// 仅显示有 PDF 的条目
        #[arg(long)]
        has_pdf: bool,

        /// 排序方式
        #[arg(long, default_value = "date")]
        sort: String,

        /// 最大返回数量
        #[arg(long, short)]
        limit: Option<u32>,
    },

    /// 查询特定 DOI 的本地文件状态
    Status {
        /// 论文 DOI
        doi: String,
    },

    /// 删除本地论文文件
    Remove {
        /// 论文 DOI
        doi: String,

        /// 跳过确认
        #[arg(long)]
        force: bool,
    },

    /// 输出 CLI schema JSON（供 LLM 自省）
    Schema {
        /// 特定子命令的 schema
        subcommand: Option<String>,
    },

    /// 生成 shell 补全脚本
    Completions {
        /// 目标 shell 类型
        shell: clap_complete::Shell,
    },

    /// 管理本地配置
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    /// 显示当前配置
    Show,

    /// 设置配置项
    Set {
        /// 设置默认邮箱
        #[arg(long)]
        email: Option<String>,

        /// 设置默认输出格式
        #[arg(long)]
        output: Option<CliOutputFormat>,

        /// 设置默认搜索返回数量
        #[arg(long)]
        limit: Option<u32>,

        /// 设置最大并发下载数
        #[arg(long)]
        max_concurrent: Option<usize>,

        /// 设置下载超时（秒）
        #[arg(long)]
        timeout_secs: Option<u64>,
    },
}
