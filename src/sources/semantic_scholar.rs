use reqwest::Client;
use serde_json::Value;

use crate::error::AppError;
use crate::models::paper::{Author, OaLocation, Paper};
use crate::sources::PaperSource;

/// Semantic Scholar API 客户端
///
/// 提供论文搜索和 DOI 查询功能，支持可选的 API Key 认证。
/// 无 Key 时有更严格的速率限制。
pub struct SemanticScholarClient {
    /// 共享的 HTTP 客户端
    client: Client,
    /// 用户邮箱，用于 User-Agent
    email: String,
    /// 可选的 API Key，设置后放宽速率限制
    api_key: Option<String>,
}

const BASE_URL: &str = "https://api.semanticscholar.org/graph/v1";

/// 搜索接口请求的字段列表
const SEARCH_FIELDS: &str =
    "title,authors,year,venue,citationCount,isOpenAccess,openAccessPdf,abstract,externalIds";

impl SemanticScholarClient {
    pub fn new(client: Client, email: &str, api_key: Option<String>) -> Self {
        Self {
            client,
            email: email.to_string(),
            api_key,
        }
    }

    /// 构建带认证信息的请求 builder
    fn build_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut builder = self.client.get(url).header(
            "User-Agent",
            format!("paperfetcher/0.1.0 (mailto:{})", self.email),
        );
        // 如果有 API Key，添加到请求头
        if let Some(ref key) = self.api_key {
            builder = builder.header("x-api-key", key);
        }
        builder
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
                source: "semantic_scholar".to_string(),
                retry_after_secs: retry_after,
            })
        } else {
            let code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(AppError::ApiError {
                source: "semantic_scholar".to_string(),
                status_code: code,
                body,
            })
        }
    }

    /// 将 Semantic Scholar 论文 JSON 解析为 Paper 结构体
    ///
    /// `fallback_doi` 用于当 JSON 中无 externalIds.DOI 时回退使用
    fn parse_paper(&self, json: &Value, fallback_doi: &str) -> Result<Paper, AppError> {
        let title = json["title"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        // DOI: 优先从 externalIds.DOI 取，否则用传入的 fallback_doi
        let doi = json["externalIds"]["DOI"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| fallback_doi.to_string());

        // 解析作者列表
        let authors = json["authors"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| {
                        let name = a["name"].as_str()?;
                        Some(Author {
                            name: name.to_string(),
                            affiliation: None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let year = json["year"].as_u64().and_then(|y| u16::try_from(y).ok());
        let venue = json["venue"].as_str().filter(|s| !s.is_empty()).map(String::from);
        let cited_by_count = json["citationCount"].as_u64();
        let is_open_access = json["isOpenAccess"].as_bool().unwrap_or(false);

        // 解析 openAccessPdf 字段
        let oa_pdf_url = json["openAccessPdf"]["url"].as_str().map(String::from);
        let oa_locations = if let Some(ref url) = oa_pdf_url {
            vec![OaLocation {
                url: url.clone(),
                host_type: None,
                license: None,
                version: None,
            }]
        } else {
            vec![]
        };

        // 摘要
        let abstract_text = json["abstract"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from);

        Ok(Paper {
            doi,
            title,
            authors,
            year,
            venue,
            publisher: None, // Semantic Scholar 不直接提供 publisher
            abstract_text,
            cited_by_count,
            is_open_access,
            oa_locations,
            best_oa_url: oa_pdf_url,
            source: "semantic_scholar".to_string(),
        })
    }
}

impl PaperSource for SemanticScholarClient {
    fn name(&self) -> &str {
        "semantic_scholar"
    }

    /// 按关键词搜索论文
    ///
    /// GET /paper/search?query={q}&limit={n}&fields=...&year={year}
    async fn search(
        &self,
        query: &str,
        limit: u32,
        year: Option<&str>,
        _open_access_only: bool,
    ) -> Result<Vec<Paper>, AppError> {
        let url = format!("{BASE_URL}/paper/search");

        // 构建查询参数
        let mut params: Vec<(&str, String)> = vec![
            ("query", query.to_string()),
            ("limit", limit.to_string()),
            ("fields", SEARCH_FIELDS.to_string()),
        ];
        // 年份过滤（Semantic Scholar 支持 "2020" 或 "2020-2024" 格式）
        if let Some(y) = year {
            params.push(("year", y.to_string()));
        }

        let response = self
            .build_request(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| {
                AppError::NetworkError(format!("Semantic Scholar search request failed: {e}"))
            })?;

        let json = self.handle_response(response, "").await?;

        // 从 data 数组中解析论文列表
        let papers = json["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| self.parse_paper(item, "").ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(papers)
    }

    /// 按 DOI 查询单篇论文
    ///
    /// GET /paper/DOI:{doi}?fields=...
    async fn lookup(&self, doi: &str) -> Result<Paper, AppError> {
        let url = format!("{BASE_URL}/paper/DOI:{doi}");
        let response = self
            .build_request(&url)
            .query(&[("fields", SEARCH_FIELDS)])
            .send()
            .await
            .map_err(|e| {
                AppError::NetworkError(format!("Semantic Scholar lookup request failed: {e}"))
            })?;

        let json = self.handle_response(response, doi).await?;
        self.parse_paper(&json, doi)
    }

    /// 获取论文的开放获取 PDF URL
    async fn get_pdf_url(&self, doi: &str) -> Result<Option<String>, AppError> {
        let paper = self.lookup(doi).await?;
        Ok(paper.best_oa_url)
    }
}
