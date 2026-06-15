use reqwest::Client;
use serde_json::Value;

use crate::error::AppError;
use crate::models::paper::{Author, OaLocation, Paper};
use crate::sources::PaperSource;

/// CrossRef API 客户端
///
/// CrossRef 是最权威的 DOI 元数据来源，
/// 支持关键词搜索和 DOI 精确查询。
/// 使用 mailto 参数进入 "polite pool" 获得更好的服务质量。
pub struct CrossRefClient {
    /// 共享的 HTTP 客户端
    client: Client,
    /// 用户邮箱，用于 polite pool
    email: String,
}

const BASE_URL: &str = "https://api.crossref.org";

impl CrossRefClient {
    pub fn new(client: Client, email: &str) -> Self {
        Self {
            client,
            email: email.to_string(),
        }
    }

    /// 统一处理 HTTP 响应状态码
    async fn handle_response(
        &self,
        response: reqwest::Response,
        doi: &str,
    ) -> Result<Value, AppError> {
        let status = response.status();
        if status.is_success() {
            let body = response.text().await.map_err(|e| {
                AppError::NetworkError(format!("Failed to read response body: {e}"))
            })?;
            serde_json::from_str(&body)
                .map_err(|e| AppError::ParseError(format!("JSON parse error: {e}")))
        } else if status.as_u16() == 404 {
            Err(AppError::NotFound {
                doi: doi.to_string(),
            })
        } else if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            Err(AppError::RateLimited {
                source: "crossref".to_string(),
                retry_after_secs: retry_after,
            })
        } else {
            let code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(AppError::ApiError {
                source: "crossref".to_string(),
                status_code: code,
                body,
            })
        }
    }

    /// 将 CrossRef message 对象解析为 Paper 结构体
    ///
    /// CrossRef 的响应体结构为 { "status": "ok", "message": { ... } }，
    /// 搜索时为 { "status": "ok", "message": { "items": [...] } }。
    fn parse_paper(&self, msg: &Value) -> Result<Paper, AppError> {
        // DOI
        let doi = msg["DOI"].as_str().unwrap_or_default().to_string();

        // 标题：CrossRef 的 title 是数组格式
        let title = msg["title"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string();

        // 解析作者列表：拼接 given + family
        let authors = msg["author"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| {
                        let given = a["given"].as_str().unwrap_or("");
                        let family = a["family"].as_str().unwrap_or("");
                        if given.is_empty() && family.is_empty() {
                            return None;
                        }
                        let name = if given.is_empty() {
                            family.to_string()
                        } else if family.is_empty() {
                            given.to_string()
                        } else {
                            format!("{given} {family}")
                        };
                        // 机构：author[].affiliation[].name
                        let affiliation = a["affiliation"]
                            .as_array()
                            .and_then(|affs| affs.first())
                            .and_then(|aff| aff["name"].as_str())
                            .map(String::from);
                        Some(Author { name, affiliation })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // 年份：从 published.date-parts[0][0] 提取
        // CrossRef 的日期结构为 { "date-parts": [[2020, 3, 15]] }
        let year = msg["published"]["date-parts"]
            .as_array()
            .and_then(|outer| outer.first())
            .and_then(|inner| inner.as_array())
            .and_then(|parts| parts.first())
            .and_then(|y| y.as_u64())
            .and_then(|y| u16::try_from(y).ok());

        // 期刊/会议名称：container-title 是数组
        let venue = msg["container-title"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|t| t.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from);

        // 出版商
        let publisher = msg["publisher"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from);

        // 被引次数
        let cited_by_count = msg["is-referenced-by-count"].as_u64();

        // 摘要
        let abstract_text = msg["abstract"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from);

        // 从 link 数组中提取 PDF 类型的链接作为 OA 位置
        let oa_locations = msg["link"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|link| {
                        let content_type = link["content-type"].as_str().unwrap_or("");
                        // 仅保留 PDF 类型的链接
                        if !content_type.contains("application/pdf") {
                            return None;
                        }
                        let url = link["URL"].as_str()?;
                        Some(OaLocation {
                            url: url.to_string(),
                            host_type: Some("publisher".to_string()),
                            license: None,
                            version: link["content-version"].as_str().map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // CrossRef 不直接标记 OA 状态，根据 license 字段推测
        let is_open_access = msg["license"]
            .as_array()
            .is_some_and(|arr| !arr.is_empty());

        let best_oa_url = oa_locations.first().map(|loc| loc.url.clone());

        Ok(Paper {
            doi,
            title,
            authors,
            year,
            venue,
            publisher,
            abstract_text,
            cited_by_count,
            is_open_access,
            oa_locations,
            best_oa_url,
            source: "crossref".to_string(),
        })
    }
}

impl PaperSource for CrossRefClient {
    fn name(&self) -> &str {
        "crossref"
    }

    /// 按关键词搜索论文
    ///
    /// GET /works?query={q}&rows={n}&mailto={email}&filter=from-pub-date:{year}
    async fn search(
        &self,
        query: &str,
        limit: u32,
        year: Option<&str>,
        _open_access_only: bool,
    ) -> Result<Vec<Paper>, AppError> {
        let url = format!("{BASE_URL}/works");

        let mut params: Vec<(&str, String)> = vec![
            ("query", query.to_string()),
            ("rows", limit.to_string()),
            ("mailto", self.email.clone()),
        ];

        // 年份过滤
        if let Some(y) = year {
            params.push(("filter", format!("from-pub-date:{y}")));
        }

        let response = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                format!("paperfetcher/0.1.0 (mailto:{})", self.email),
            )
            .query(&params)
            .send()
            .await
            .map_err(|e| {
                AppError::NetworkError(format!("CrossRef search request failed: {e}"))
            })?;

        let json = self.handle_response(response, "").await?;

        // CrossRef 搜索结果在 message.items 数组下
        let papers = json["message"]["items"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| self.parse_paper(item).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(papers)
    }

    /// 按 DOI 查询单篇论文
    ///
    /// GET /works/{doi}?mailto={email}
    async fn lookup(&self, doi: &str) -> Result<Paper, AppError> {
        let url = format!("{BASE_URL}/works/{doi}");
        let response = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                format!("paperfetcher/0.1.0 (mailto:{})", self.email),
            )
            .query(&[("mailto", &self.email)])
            .send()
            .await
            .map_err(|e| {
                AppError::NetworkError(format!("CrossRef lookup request failed: {e}"))
            })?;

        let json = self.handle_response(response, doi).await?;

        // 单篇查询结果在 message 对象下
        self.parse_paper(&json["message"])
    }

    /// CrossRef 通常不直接提供 PDF URL，返回 None
    async fn get_pdf_url(&self, doi: &str) -> Result<Option<String>, AppError> {
        let paper = self.lookup(doi).await?;
        Ok(paper.best_oa_url)
    }
}
