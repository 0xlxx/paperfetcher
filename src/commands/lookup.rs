// lookup 子命令 — 按 DOI 查询单篇论文详情
use crate::config::Config;
use crate::error::AppError;
use crate::models::response::LookupResponse;
use crate::output::eprintln_info;
use crate::sources::{self, SourceName, PaperSource};

/// 执行查询命令
///
/// 通过 DOI 从指定数据源获取论文完整元数据
pub async fn execute(
    source_name: SourceName,
    doi: &str,
    config: &Config,
) -> Result<LookupResponse, AppError> {
    // 校验 DOI 格式（基本检查）
    if !doi.contains('/') {
        return Err(AppError::InvalidDoi {
            doi: doi.to_string(),
        });
    }

    eprintln_info(&format!("looking up DOI '{doi}' from {source_name}"));

    let client = reqwest::Client::builder()
        .user_agent("paperfetcher/0.1.0 (academic-paper-cli)")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(AppError::NetworkError)?;

    let source = sources::create_source(source_name, &client, &config.email);
    let paper = source.lookup(doi).await?;

    Ok(LookupResponse {
        source: source.name().to_string(),
        paper,
    })
}
