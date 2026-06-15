// PDF 下载器 — 管理论文 PDF 的下载和本地存储
use std::path::PathBuf;
use std::time::Duration;

use crate::error::AppError;
use crate::models::paper::{FetchAction, FetchResult, Paper};

/// PDF 下载器
pub struct Downloader {
    /// HTTP 客户端
    client: reqwest::Client,
    /// 数据存储目录
    data_dir: PathBuf,
    /// 最大并发数（预留给 batch 下载）
    #[allow(dead_code)]
    max_concurrent: usize,
    /// 下载超时时间
    timeout: Duration,
}

impl Downloader {
    /// 构造下载器
    pub fn new(
        client: reqwest::Client,
        data_dir: PathBuf,
        max_concurrent: usize,
        timeout: Duration,
    ) -> Self {
        Self {
            client,
            data_dir,
            max_concurrent,
            timeout,
        }
    }

    /// 下载单篇论文 PDF
    ///
    /// 流程：检查文件是否存在 → 下载 PDF → 保存元数据 → 返回结果
    pub async fn download_paper(
        &self,
        doi: &str,
        pdf_url: &str,
        overwrite: bool,
        metadata: Option<&Paper>,
    ) -> Result<FetchResult, AppError> {
        let filename = Self::doi_to_filename(doi);
        let papers_dir = self.paper_dir();
        std::fs::create_dir_all(&papers_dir)?;

        let pdf_path = papers_dir.join(format!("{filename}.pdf"));

        // 如果文件已存在且不覆盖，则跳过
        if pdf_path.exists() && !overwrite {
            return Ok(FetchResult {
                doi: doi.to_string(),
                changed: false,
                action: FetchAction::Skipped,
                path: Some(pdf_path.to_string_lossy().into_owned()),
                metadata_path: None,
                size_bytes: pdf_path.metadata().ok().map(|m| m.len()),
                source: None,
                reason: Some("file already exists".to_string()),
            });
        }

        // 发起 HTTP 请求下载 PDF
        let response = self
            .client
            .get(pdf_url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(AppError::NetworkError)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            return Ok(FetchResult {
                doi: doi.to_string(),
                changed: false,
                action: FetchAction::Failed,
                path: None,
                metadata_path: None,
                size_bytes: None,
                source: None,
                reason: Some(format!("HTTP {status} from {pdf_url}")),
            });
        }

        // 读取响应体并写入文件
        let bytes = response.bytes().await.map_err(|e| {
            AppError::NetworkError(e)
        })?;
        let size = bytes.len() as u64;
        std::fs::write(&pdf_path, &bytes)?;

        // 保存元数据 JSON（如果提供）
        let metadata_path = if let Some(paper) = metadata {
            let meta_path = papers_dir.join(format!("{filename}.json"));
            let meta_json = serde_json::to_string_pretty(paper)?;
            std::fs::write(&meta_path, meta_json)?;
            Some(meta_path.to_string_lossy().into_owned())
        } else {
            None
        };

        Ok(FetchResult {
            doi: doi.to_string(),
            changed: true,
            action: FetchAction::Downloaded,
            path: Some(pdf_path.to_string_lossy().into_owned()),
            metadata_path,
            size_bytes: Some(size),
            source: None,
            reason: None,
        })
    }

    /// 将 DOI 转换为安全的文件名（/ → _）
    pub fn doi_to_filename(doi: &str) -> String {
        doi.replace('/', "_")
            .replace('\\', "_")
            .replace(':', "_")
            .replace('*', "_")
            .replace('?', "_")
            .replace('"', "_")
            .replace('<', "_")
            .replace('>', "_")
            .replace('|', "_")
    }

    /// 论文文件存储目录
    pub fn paper_dir(&self) -> PathBuf {
        self.data_dir.join("papers")
    }
}
