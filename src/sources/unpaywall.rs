use reqwest::Client;
use serde_json::Value;

use crate::error::AppError;
use crate::models::paper::{Author, OaLocation, Paper};
use crate::sources::PaperSource;

/// Unpaywall API 客户端
///
/// Unpaywall 仅支持按 DOI 查询，不支持关键词搜索。
/// 主要用于获取论文的开放获取 PDF 链接。
pub struct UnpaywallClient {
    /// 共享的 HTTP 客户端
    client: Client,
    /// 用户邮箱，Unpaywall 要求作为认证参数
    email: String,
}

const BASE_URL: &str = "https://api.unpaywall.org/v2";

impl UnpaywallClient {
    pub fn new(client: Client, email: &str) -> Self {
        Self {
            client,
            email: email.to_string(),
        }
    }

    /// 将 Unpaywall JSON 响应解析为 Paper 结构体
    fn parse_paper(&self, json: &Value, doi: &str) -> Result<Paper, AppError> {
        // 提取标题，缺失时使用空字符串
        let title = json["title"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        // 解析作者列表：将 given + family 拼接为完整姓名
        let authors = json["z_authors"]
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
                        Some(Author {
                            name,
                            affiliation: None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // 发表年份
        let year = json["year"].as_u64().and_then(|y| u16::try_from(y).ok());

        // 期刊名称
        let venue = json["journal_name"].as_str().map(String::from);

        // 出版商
        let publisher = json["publisher"].as_str().map(String::from);

        // 是否开放获取
        let is_open_access = json["is_oa"].as_bool().unwrap_or(false);

        // 解析所有开放获取位置
        let oa_locations = json["oa_locations"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|loc| {
                        // 优先使用 url_for_pdf，其次使用 url
                        let url = loc["url_for_pdf"]
                            .as_str()
                            .or_else(|| loc["url"].as_str())?;
                        Some(OaLocation {
                            url: url.to_string(),
                            host_type: loc["host_type"].as_str().map(String::from),
                            license: loc["license"].as_str().map(String::from),
                            version: loc["version"].as_str().map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // 最佳开放获取 URL：优先取 best_oa_location.url_for_pdf
        let best_oa_url = json["best_oa_location"]["url_for_pdf"]
            .as_str()
            .or_else(|| json["best_oa_location"]["url"].as_str())
            .map(String::from);

        Ok(Paper {
            doi: doi.to_string(),
            title,
            authors,
            year,
            venue,
            publisher,
            abstract_text: None, // Unpaywall 不提供摘要
            cited_by_count: None, // Unpaywall 不提供引用数
            is_open_access,
            oa_locations,
            best_oa_url,
            source: "unpaywall".to_string(),
        })
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
            // 解析 Retry-After 头部
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            Err(AppError::RateLimited {
                source: "unpaywall".to_string(),
                retry_after_secs: retry_after,
            })
        } else {
            let code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(AppError::ApiError {
                source: "unpaywall".to_string(),
                status_code: code,
                body,
            })
        }
    }
}

impl PaperSource for UnpaywallClient {
    fn name(&self) -> &str {
        "unpaywall"
    }

    /// Unpaywall 不支持关键词搜索，直接返回错误
    async fn search(
        &self,
        _query: &str,
        _limit: u32,
        _year: Option<&str>,
        _open_access_only: bool,
    ) -> Result<Vec<Paper>, AppError> {
        Err(AppError::ApiError {
            source: "unpaywall".to_string(),
            status_code: 0,
            body: "Unpaywall does not support keyword search. Use lookup(doi) instead.".to_string(),
        })
    }

    /// 按 DOI 查询单篇论文: GET /v2/{doi}?email={email}
    async fn lookup(&self, doi: &str) -> Result<Paper, AppError> {
        let url = format!("{BASE_URL}/{doi}");
        let response = self
            .client
            .get(&url)
            .header(
                "User-Agent",
                format!("paperfetcher/0.1.0 (mailto:{})", self.email),
            )
            .query(&[("email", &self.email)])
            .send()
            .await
            .map_err(|e| AppError::NetworkError(format!("Unpaywall request failed: {e}")))?;

        let json = self.handle_response(response, doi).await?;
        self.parse_paper(&json, doi)
    }

    /// 获取论文的开放获取 PDF URL
    async fn get_pdf_url(&self, doi: &str) -> Result<Option<String>, AppError> {
        let paper = self.lookup(doi).await?;
        Ok(paper.best_oa_url)
    }
}
