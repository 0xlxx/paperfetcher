// 本地索引管理 — JSON 文件存储论文索引
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::models::paper::LocalPaperEntry;

/// 本地论文索引，管理已下载论文的元数据
pub struct LocalIndex {
    /// 索引条目列表
    entries: Vec<LocalPaperEntry>,
    /// 索引文件路径
    index_path: PathBuf,
}

impl LocalIndex {
    /// 加载索引文件，不存在则创建空索引
    pub fn load(data_dir: &Path) -> Result<Self, AppError> {
        let index_path = data_dir.join("index.json");

        let entries = if index_path.exists() {
            let content = std::fs::read_to_string(&index_path)?;
            serde_json::from_str::<Vec<LocalPaperEntry>>(&content).map_err(|e| {
                AppError::ParseError(format!(
                    "failed to parse index file {}: {e}",
                    index_path.display()
                ))
            })?
        } else {
            // 确保数据目录存在
            if let Some(parent) = index_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            Vec::new()
        };

        Ok(Self {
            entries,
            index_path,
        })
    }

    /// 持久化索引到磁盘
    pub fn save(&self) -> Result<(), AppError> {
        // 确保父目录存在
        if let Some(parent) = self.index_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(&self.index_path, content)?;
        Ok(())
    }

    /// 添加或更新条目（以 DOI 去重）
    pub fn add_entry(&mut self, entry: LocalPaperEntry) {
        // 如果已存在相同 DOI 的条目，则替换
        let doi = entry.doi.clone();
        if let Some(existing) = self.entries.iter_mut().find(|e| e.doi == doi) {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// 按 DOI 删除条目，返回被删除的条目
    pub fn remove_entry(&mut self, doi: &str) -> Option<LocalPaperEntry> {
        let normalized = normalize_doi(doi);
        if let Some(pos) = self
            .entries
            .iter()
            .position(|e| normalize_doi(&e.doi) == normalized)
        {
            Some(self.entries.remove(pos))
        } else {
            None
        }
    }

    /// 按 DOI 精确查找
    pub fn find_by_doi(&self, doi: &str) -> Option<&LocalPaperEntry> {
        let normalized = normalize_doi(doi);
        self.entries
            .iter()
            .find(|e| normalize_doi(&e.doi) == normalized)
    }

    /// 模糊搜索（匹配标题或作者名）
    #[allow(dead_code)]
    pub fn search(&self, query: &str) -> Vec<&LocalPaperEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                // 匹配标题
                let title_match = e.title.to_lowercase().contains(&query_lower);
                // 匹配作者
                let author_match = e
                    .authors
                    .iter()
                    .any(|a| a.to_lowercase().contains(&query_lower));
                // 匹配 DOI
                let doi_match = e.doi.to_lowercase().contains(&query_lower);
                title_match || author_match || doi_match
            })
            .collect()
    }

    /// 带过滤条件的列表查询
    pub fn list(
        &self,
        filter: Option<&str>,
        year: Option<&str>,
        limit: Option<u32>,
    ) -> Vec<&LocalPaperEntry> {
        let mut results: Vec<&LocalPaperEntry> = self
            .entries
            .iter()
            .filter(|e| {
                // 关键词过滤
                if let Some(f) = filter {
                    let f_lower = f.to_lowercase();
                    let title_match = e.title.to_lowercase().contains(&f_lower);
                    let author_match = e
                        .authors
                        .iter()
                        .any(|a| a.to_lowercase().contains(&f_lower));
                    if !title_match && !author_match {
                        return false;
                    }
                }
                // 年份过滤
                if let Some(y) = year {
                    if let Ok(year_num) = y.parse::<u32>() {
                        if e.year != Some(year_num as u16) {
                            return false;
                        }
                    }
                }
                true
            })
            .collect();

        // 按下载时间降序排列
        results.sort_by(|a, b| b.downloaded_at.cmp(&a.downloaded_at));

        // 限制数量
        if let Some(lim) = limit {
            results.truncate(lim as usize);
        }

        results
    }

    /// 获取索引中的总条目数
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 索引是否为空
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 标准化 DOI（小写，去除首尾空白）
fn normalize_doi(doi: &str) -> String {
    doi.trim().to_lowercase()
}
