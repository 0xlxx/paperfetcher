// 数据源模块 — 学术 API 抽象层
use crate::error::AppError;
use crate::models::paper::Paper;
use serde::{Deserialize, Serialize};

/// 论文数据源 trait — 所有学术 API 的统一抽象
pub trait PaperSource: Send + Sync {
    /// 数据源名称
    fn name(&self) -> &str;

    /// 搜索论文
    async fn search(
        &self,
        query: &str,
        limit: u32,
        year: Option<&str>,
        open_access_only: bool,
    ) -> Result<Vec<Paper>, AppError>;

    /// 按 DOI 查询单篇论文
    async fn lookup(
        &self,
        doi: &str,
    ) -> Result<Paper, AppError>;

    /// 获取 PDF 下载链接
    async fn get_pdf_url(
        &self,
        doi: &str,
    ) -> Result<Option<String>, AppError>;
}

/// 支持的数据源名称枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceName {
    Unpaywall,
    SemanticScholar,
    OpenAlex,
    CrossRef,
}

impl Default for SourceName {
    fn default() -> Self {
        SourceName::OpenAlex
    }
}

impl std::fmt::Display for SourceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceName::Unpaywall => write!(f, "unpaywall"),
            SourceName::SemanticScholar => write!(f, "semantic_scholar"),
            SourceName::OpenAlex => write!(f, "openalex"),
            SourceName::CrossRef => write!(f, "crossref"),
        }
    }
}

impl std::str::FromStr for SourceName {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unpaywall" => Ok(SourceName::Unpaywall),
            "semantic_scholar" | "semanticscholar" | "s2" => Ok(SourceName::SemanticScholar),
            "openalex" => Ok(SourceName::OpenAlex),
            "crossref" => Ok(SourceName::CrossRef),
            other => Err(format!("unknown source: '{other}'")),
        }
    }
}

// ---------------------------------------------------------------------------
// OpenAlex 数据源实现
// ---------------------------------------------------------------------------

/// OpenAlex 数据源
pub struct OpenAlexSource {
    client: reqwest::Client,
    email: String,
}

impl OpenAlexSource {
    pub fn new(client: &reqwest::Client, email: &str) -> Self {
        Self {
            client: client.clone(),
            email: email.to_string(),
        }
    }

    /// 构建带 mailto 参数的 URL（OpenAlex 礼貌池）
    fn polite_url(&self, base: &str) -> String {
        if base.contains('?') {
            format!("{base}&mailto={}", self.email)
        } else {
            format!("{base}?mailto={}", self.email)
        }
    }
}

impl PaperSource for OpenAlexSource {
    fn name(&self) -> &str {
        "openalex"
    }

    async fn search(
        &self,
        query: &str,
        limit: u32,
        year: Option<&str>,
        open_access_only: bool,
    ) -> Result<Vec<Paper>, AppError> {
        // 构建 OpenAlex 搜索 URL
        let mut url = format!(
            "https://api.openalex.org/works?search={}&per_page={}",
            urlencoding(query),
            limit.min(200)
        );

        // 添加年份过滤器
        if let Some(y) = year {
            url.push_str(&format!("&filter=publication_year:{y}"));
            if open_access_only {
                url.push_str(",is_oa:true");
            }
        } else if open_access_only {
            url.push_str("&filter=is_oa:true");
        }

        let url = self.polite_url(&url);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let status_code = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::ApiError {
                source: reqwest::Client::new().get("http://localhost").build().unwrap_err(), // Temporary dummy error
                status_code,
                body,
            });
        }

        let body: serde_json::Value = resp.json().await?;
        // 解析 OpenAlex 响应结构
        let results = body["results"]
            .as_array()
            .map(|arr| arr.iter().filter_map(parse_openalex_work).collect())
            .unwrap_or_default();

        Ok(results)
    }

    async fn lookup(&self, doi: &str) -> Result<Paper, AppError> {
        let url = self.polite_url(&format!("https://api.openalex.org/works/doi:{doi}"));
        let resp = self.client.get(&url).send().await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::NotFound {
                doi: doi.to_string(),
            });
        }

        if !resp.status().is_success() {
            let status_code = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::ApiError {
                source: reqwest::Client::new().get("http://localhost").build().unwrap_err(), // Temporary dummy
                status_code,
                body,
            });
        }

        let body: serde_json::Value = resp.json().await?;
        parse_openalex_work(&body).ok_or_else(|| AppError::ParseError(
            "failed to parse OpenAlex response".to_string(),
        ))
    }

    async fn get_pdf_url(&self, doi: &str) -> Result<Option<String>, AppError> {
        let paper = self.lookup(doi).await?;
        Ok(paper.best_oa_url)
    }
}

/// 解析 OpenAlex 的 work JSON 对象为 Paper
fn parse_openalex_work(work: &serde_json::Value) -> Option<Paper> {
    let doi_raw = work["doi"].as_str().unwrap_or_default();
    // OpenAlex DOI 格式: https://doi.org/10.xxxx/yyyy → 提取纯 DOI
    let doi = doi_raw
        .strip_prefix("https://doi.org/")
        .unwrap_or(doi_raw)
        .to_string();

    if doi.is_empty() {
        return None;
    }

    let title = work["title"].as_str().unwrap_or("").to_string();

    // 解析作者列表
    let authors = work["authorships"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let name = a["author"]["display_name"].as_str()?;
                    let affiliation = a["institutions"]
                        .as_array()
                        .and_then(|insts| insts.first())
                        .and_then(|inst| inst["display_name"].as_str())
                        .map(String::from);
                    Some(crate::models::paper::Author {
                        name: name.to_string(),
                        affiliation,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let year = work["publication_year"].as_u64().map(|y| y as u32);
    let venue = work["primary_location"]["source"]["display_name"]
        .as_str()
        .map(String::from);
    let publisher = work["primary_location"]["source"]["host_organization_name"]
        .as_str()
        .map(String::from);
    let abstract_text = work["abstract_inverted_index"]
        .as_object()
        .map(|_| "[abstract available via API]".to_string());
    let cited_by_count = work["cited_by_count"].as_u64();
    let is_open_access = work["open_access"]["is_oa"].as_bool().unwrap_or(false);

    // 解析 OA 位置
    let oa_locations = work["locations"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|loc| {
                    let url = loc["pdf_url"]
                        .as_str()
                        .or_else(|| loc["landing_page_url"].as_str())?;
                    Some(crate::models::paper::OaLocation {
                        url: url.to_string(),
                        host_type: loc["source"]["type"].as_str().map(String::from),
                        license: loc["license"].as_str().map(String::from),
                        version: loc["version"].as_str().map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // 最佳 PDF URL: 优先 best_oa_location 的 pdf_url
    let best_oa_url = work["best_oa_location"]["pdf_url"]
        .as_str()
        .or_else(|| work["open_access"]["oa_url"].as_str())
        .map(String::from);

    Some(Paper {
        doi,
        title,
        authors,
        year: year.map(|y| y as u16),
        venue,
        publisher,
        abstract_text,
        cited_by_count,
        is_open_access,
        oa_locations,
        best_oa_url,
        source: "openalex".to_string(),
    })
}

/// 简单 URL 编码（替代 percent-encoding crate）
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '&' => "%26".to_string(),
            '=' => "%3D".to_string(),
            '+' => "%2B".to_string(),
            '#' => "%23".to_string(),
            _ if c.is_ascii_alphanumeric() || "-._~".contains(c) => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

pub enum SourceClient {
    OpenAlex(OpenAlexSource),
}

impl PaperSource for SourceClient {
    fn name(&self) -> &str {
        match self {
            Self::OpenAlex(c) => c.name(),
        }
    }

    async fn search(&self, query: &str, limit: u32, year: Option<&str>, open_access_only: bool) -> Result<Vec<Paper>, AppError> {
        match self {
            Self::OpenAlex(c) => c.search(query, limit, year, open_access_only).await,
        }
    }

    async fn lookup(&self, doi: &str) -> Result<Paper, AppError> {
        match self {
            Self::OpenAlex(c) => c.lookup(doi).await,
        }
    }

    async fn get_pdf_url(&self, doi: &str) -> Result<Option<String>, AppError> {
        match self {
            Self::OpenAlex(c) => c.get_pdf_url(doi).await,
        }
    }
}

/// 工厂函数 — 根据名称创建对应数据源实例
pub fn create_source(
    name: SourceName,
    client: &reqwest::Client,
    email: &str,
) -> SourceClient {
    match name {
        // 目前统一使用 OpenAlex 实现，后续可扩展独立实现
        SourceName::OpenAlex => SourceClient::OpenAlex(OpenAlexSource::new(client, email)),
        SourceName::Unpaywall => SourceClient::OpenAlex(OpenAlexSource::new(client, email)),
        SourceName::SemanticScholar => SourceClient::OpenAlex(OpenAlexSource::new(client, email)),
        SourceName::CrossRef => SourceClient::OpenAlex(OpenAlexSource::new(client, email)),
    }
}
