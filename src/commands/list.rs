// list 子命令 — 列出本地已下载的论文
use crate::config::Config;
use crate::error::AppError;
use crate::models::response::ListResponse;
use crate::storage::index::LocalIndex;

/// 执行列表命令
///
/// 从本地索引读取论文条目，支持过滤和分页
pub fn execute(
    filter: Option<&str>,
    year: Option<&str>,
    limit: Option<u32>,
    config: &Config,
) -> Result<ListResponse, AppError> {
    let index = LocalIndex::load(&config.data_dir)?;

    let papers = index.list(filter, year, limit);
    let total = papers.len() as u32;

    // 克隆结果用于返回
    let papers_owned = papers.into_iter().cloned().collect();

    Ok(ListResponse {
        total,
        papers: papers_owned,
    })
}
