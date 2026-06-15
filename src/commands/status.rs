// status 子命令 — 查询特定 DOI 的本地文件状态
use crate::config::Config;
use crate::error::AppError;
use crate::models::paper::PaperStatus;
use crate::storage::download::Downloader;
use crate::storage::index::LocalIndex;

/// 执行状态查询命令
///
/// 检查本地索引和文件系统，返回论文的完整本地状态
pub fn execute(doi: &str, config: &Config) -> Result<PaperStatus, AppError> {
    let index = LocalIndex::load(&config.data_dir)?;

    // 从索引中查找
    let entry = index.find_by_doi(doi);

    // 检查实际文件状态
    let filename = Downloader::doi_to_filename(doi);
    let papers_dir = config.data_dir.join("papers");
    let pdf_path = papers_dir.join(format!("{filename}.pdf"));
    let metadata_path = papers_dir.join(format!("{filename}.json"));

    let has_pdf = pdf_path.exists();
    let pdf_size = if has_pdf {
        pdf_path.metadata().ok().map(|m| m.len())
    } else {
        None
    };
    let has_metadata = metadata_path.exists();

    Ok(PaperStatus {
        doi: doi.to_string(),
        has_pdf,
        pdf_path: if has_pdf {
            Some(pdf_path.to_string_lossy().into_owned())
        } else {
            None
        },
        pdf_size_bytes: pdf_size,
        has_metadata,
        metadata_path: if has_metadata {
            Some(metadata_path.to_string_lossy().into_owned())
        } else {
            None
        },
        downloaded_at: entry.map(|e| e.downloaded_at.clone()),
    })
}
