// search 子命令 — 搜索学术论文
use crate::config::Config;
use crate::error::AppError;
use crate::models::response::SearchResponse;
use crate::output::eprintln_info;
use crate::sources::{self, SourceName, PaperSource};

/// 执行搜索命令
///
/// 创建 HTTP 客户端和数据源，调用 API 搜索论文并返回结构化结果
pub async fn execute(
    source_name: SourceName,
    query: &str,
    limit: u32,
    year: Option<&str>,
    open_access_only: bool,
    config: &Config,
) -> Result<SearchResponse, AppError> {
    eprintln_info(&format!(
        "searching {source_name} for '{query}' (limit={limit})"
    ));

    // 创建 HTTP 客户端
    let client = build_http_client()?;

    // 创建数据源实例
    let source = sources::create_source(source_name, &client, &config.email);

    // 执行搜索
    let results = source.search(query, limit, year, open_access_only).await?;
    let returned = results.len() as u32;

    eprintln_info(&format!("found {returned} results"));

    Ok(SearchResponse {
        source: source.name().to_string(),
        query: query.to_string(),
        total_results: returned as u64,
        returned,
        results,
    })
}

/// 构建 HTTP 客户端（带合理默认配置）
fn build_http_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .user_agent("paperfetcher/0.1.0 (academic-paper-cli)")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(AppError::NetworkError)
}
