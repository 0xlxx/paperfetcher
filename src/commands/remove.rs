// remove 子命令 — 删除本地论文文件
use crate::config::Config;
use crate::error::AppError;
use crate::models::response::RemoveResponse;
use crate::storage::download::Downloader;
use crate::storage::index::LocalIndex;

/// 执行删除命令
///
/// 删除 PDF 和元数据文件，同时从索引中移除条目
pub fn execute(doi: &str, config: &Config) -> Result<RemoveResponse, AppError> {
    let filename = Downloader::doi_to_filename(doi);
    let papers_dir = config.data_dir.join("papers");
    let pdf_path = papers_dir.join(format!("{filename}.pdf"));
    let metadata_path = papers_dir.join(format!("{filename}.json"));

    // 尝试删除 PDF 文件
    let removed_pdf = if pdf_path.exists() {
        std::fs::remove_file(&pdf_path)?;
        true
    } else {
        false
    };

    // 尝试删除元数据文件
    let removed_metadata = if metadata_path.exists() {
        std::fs::remove_file(&metadata_path)?;
        true
    } else {
        false
    };

    // 从索引中移除
    let mut index = LocalIndex::load(&config.data_dir)?;
    let _removed_entry = index.remove_entry(doi);
    index.save()?;

    let changed = removed_pdf || removed_metadata;

    Ok(RemoveResponse {
        doi: doi.to_string(),
        removed_pdf,
        removed_metadata,
        changed,
    })
}
