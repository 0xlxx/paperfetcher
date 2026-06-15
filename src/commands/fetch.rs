// fetch 子命令 — 下载论文 PDF（支持单个 DOI、文件批量、stdin 输入）
use std::io::BufRead;
use std::path::Path;
use std::time::Duration;

use crate::config::Config;
use crate::error::AppError;
use crate::models::paper::{FetchAction, FetchResult, FetchSummary, LocalPaperEntry};
use crate::models::response::FetchResponse;
use crate::output::eprintln_info;
use crate::sources::{self, SourceName, PaperSource};
use crate::storage::download::Downloader;
use crate::storage::index::LocalIndex;

/// 执行下载命令
///
/// 支持三种输入方式：
/// 1. 单个 DOI 字符串
/// 2. 包含 DOI 列表的文件（每行一个 DOI）
/// 3. 从 stdin 读取 DOI 列表（--stdin）
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    doi_or_file: Option<&str>,
    from_stdin: bool,
    overwrite: bool,
    with_metadata: bool,
    max_concurrent: usize,
    timeout: u64,
    source_names: &[SourceName],
    config: &Config,
) -> Result<FetchResponse, AppError> {
    // 第一步：解析 DOI 列表
    let dois = parse_doi_input(doi_or_file, from_stdin)?;

    if dois.is_empty() {
        return Ok(FetchResponse {
            results: Vec::new(),
            summary: FetchSummary {
                total: 0,
                downloaded: 0,
                skipped: 0,
                failed: 0,
            },
        });
    }

    eprintln_info(&format!("preparing to fetch {} paper(s)", dois.len()));

    // 第二步：初始化下载器和数据源
    let client = reqwest::Client::builder()
        .user_agent("paperfetcher/0.1.0 (academic-paper-cli)")
        .timeout(Duration::from_secs(timeout))
        .build()
        .map_err(AppError::NetworkError)?;

    // 确定使用的数据源（优先使用命令行指定的，否则使用配置默认值）
    let effective_sources = if source_names.is_empty() {
        config.sources.pdf_sources.clone()
    } else {
        source_names.to_vec()
    };

    let downloader = Downloader::new(
        client.clone(),
        config.data_dir.clone(),
        max_concurrent,
        Duration::from_secs(timeout),
    );

    // 加载本地索引
    let mut index = LocalIndex::load(&config.data_dir)?;
    let mut results = Vec::new();

    // 第三步：逐个处理 DOI（回退策略获取 PDF URL）
    for doi in &dois {
        eprintln_info(&format!("processing: {doi}"));

        let result = fetch_single_paper(
            doi,
            &client,
            &config.email,
            &effective_sources,
            &downloader,
            overwrite,
            with_metadata,
        )
        .await;

        match result {
            Ok(fetch_result) => {
                // 如果下载成功，更新索引
                if fetch_result.action == FetchAction::Downloaded {
                    let entry = LocalPaperEntry {
                        doi: doi.clone(),
                        title: String::new(), // 如果有元数据会在后续更新
                        authors: Vec::new(),
                        year: None,
                        pdf_path: fetch_result.path.clone().unwrap_or_default(),
                        metadata_path: fetch_result.metadata_path.clone(),
                        downloaded_at: chrono::Utc::now().to_rfc3339(),
                        size_bytes: fetch_result.size_bytes.unwrap_or(0),
                    };
                    index.add_entry(entry);
                }
                results.push(fetch_result);
            }
            Err(e) => {
                results.push(FetchResult {
                    doi: doi.clone(),
                    changed: false,
                    action: FetchAction::Failed,
                    path: None,
                    metadata_path: None,
                    size_bytes: None,
                    source: None,
                    reason: Some(e.to_string()),
                });
            }
        }
    }

    // 保存索引
    index.save()?;

    // 统计摘要
    let summary = FetchSummary {
        total: results.len() as u32,
        downloaded: results
            .iter()
            .filter(|r| r.action == FetchAction::Downloaded)
            .count() as u32,
        skipped: results
            .iter()
            .filter(|r| r.action == FetchAction::Skipped)
            .count() as u32,
        failed: results
            .iter()
            .filter(|r| r.action == FetchAction::Failed)
            .count() as u32,
    };

    eprintln_info(&format!(
        "fetch complete: {} downloaded, {} skipped, {} failed",
        summary.downloaded, summary.skipped, summary.failed
    ));

    Ok(FetchResponse { results, summary })
}

/// 下载单篇论文（多数据源回退策略）
async fn fetch_single_paper(
    doi: &str,
    client: &reqwest::Client,
    email: &str,
    source_names: &[SourceName],
    downloader: &Downloader,
    overwrite: bool,
    with_metadata: bool,
) -> Result<FetchResult, AppError> {
    let mut last_error: Option<AppError> = None;

    // 依次尝试每个数据源获取 PDF URL
    for &source_name in source_names {
        let source = sources::create_source(source_name, client, email);
        eprintln_info(&format!("  trying source: {}", source.name()));

        // 尝试获取 PDF URL
        match source.get_pdf_url(doi).await {
            Ok(Some(url)) => {
                // 如果需要元数据，尝试获取论文信息
                let metadata = if with_metadata {
                    source.lookup(doi).await.ok()
                } else {
                    None
                };

                let mut result = downloader
                    .download_paper(doi, &url, overwrite, metadata.as_ref())
                    .await?;
                result.source = Some(source.name().to_string());
                return Ok(result);
            }
            Ok(None) => {
                eprintln_info(&format!(
                    "  no PDF URL found from {}",
                    source.name()
                ));
                continue;
            }
            Err(e) => {
                eprintln_info(&format!(
                    "  error from {}: {e}",
                    source.name()
                ));
                last_error = Some(e);
                continue;
            }
        }
    }

    // 所有数据源都失败
    Err(last_error.unwrap_or_else(|| AppError::NotFound {
        doi: doi.to_string(),
    }))
}

/// 解析 DOI 输入（单个 DOI、文件路径或 stdin）
fn parse_doi_input(doi_or_file: Option<&str>, from_stdin: bool) -> Result<Vec<String>, AppError> {
    if from_stdin {
        // 从 stdin 读取
        let stdin = std::io::stdin();
        let dois: Vec<String> = stdin
            .lock()
            .lines()
            .filter_map(|line| {
                let line = line.ok()?;
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    None
                } else {
                    Some(trimmed)
                }
            })
            .collect();
        return Ok(dois);
    }

    let doi_or_file = doi_or_file.unwrap_or_default();
    
    // 检查是否是文件路径
    let path = Path::new(doi_or_file);
    if path.exists() && path.is_file() {
        let content = std::fs::read_to_string(path)?;
        let dois: Vec<String> = content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        return Ok(dois);
    }

    // 当作单个 DOI 处理
    Ok(vec![doi_or_file.to_string()])
}
