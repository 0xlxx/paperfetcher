use reqwest::Client;
use serde_json::Value;

use crate::error::AppError;
use crate::models::paper::{Author, OaLocation, Paper};
use crate::sources::PaperSource;

/// OpenAlex API 客户端
///
/// OpenAlex 提供丰富的开放学术元数据，
/// 支持关键词搜索、DOI 查询，以及按年份和 OA 状态过滤。
pub struct OpenAlexClient {
    /// 共享的 HTTP 客户端
    client: Client,
    /// 用户邮箱，用于 mailto 参数进入 polite pool
    email: String,
}

const BASE_URL: &str = "https://api.openalex.org";

impl OpenAlexClient {
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
                source: "openalex".to_string(),
                retry_after_secs: retry_after,
            })
        } else {
            let code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(AppError::ApiError {
                source: "openalex".to_string(),
                status_code: code,
                body,
            })
        }
    }

    /// 去掉 OpenAlex 返回的 DOI 完整 URL 前缀
    ///
    /// OpenAlex 的 doi 字段格式为 "https://doi.org/10.xxx"，
    /// 需要去除前缀只保留 "10.xxx" 部分。
    fn normalize_doi(raw_doi: &str) -> String {
        raw_doi
            .strip_prefix("https://doi.org/")
            .unwrap_or(raw_doi)
            .to_string()
    }

    /// 将 OpenAlex 论文 JSON 解析为 Paper 结构体
    fn parse_paper(&self, json: &Value) -> Result<Paper, AppError> {
        // 提取并规范化 DOI
        let doi = json["doi"]
            .as_str()
            .map(Self::normalize_doi)
            .unwrap_or_default();

        let title = json["display_name"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        // 解析作者和机构信息
        // authorships[].author.display_name → name
        // authorships[].institutions[0].display_name → affiliation
        let authors = json["authorships"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|authorship| {
                        let name = authorship["author"]["display_name"].as_str()?;
                        let affiliation = authorship["institutions"]
                            .as_array()
                            .and_then(|insts| insts.first())
                            .and_then(|inst| inst["display_name"].as_str())
                            .map(String::from);
                        Some(Author {
                            name: name.to_string(),
                            affiliation,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let year = json["publication_year"]
            .as_u64()
            .and_then(|y| u16::try_from(y).ok());

        // 期刊/会议名称：从 primary_location.source.display_name 获取
        let venue = json["primary_location"]["source"]["display_name"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from);

        // 出版商
        let publisher = json["host_organization_name"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from);

        // 被引次数
        let cited_by_count = json["cited_by_count"].as_u64();

        // 开放获取状态
        let is_open_access = json["open_access"]["is_oa"].as_bool().unwrap_or(false);

        // 最佳 OA URL
        let best_oa_url = json["open_access"]["oa_url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from);

        // 从 locations 解析所有 OA 位置
        let oa_locations = json["locations"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|loc| {
                        // 仅包含有 is_oa=true 的位置
                        if !loc["is_oa"].as_bool().unwrap_or(false) {
                            return None;
                        }
                        let url = loc["pdf_url"]
                            .as_str()
                            .or_else(|| loc["landing_page_url"].as_str())?;
                        Some(OaLocation {
                            url: url.to_string(),
                            host_type: loc["source"]["type"].as_str().map(String::from),
                            license: loc["license"].as_str().map(String::from),
                            version: loc["version"].as_str().map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // 摘要：OpenAlex 使用 abstract_inverted_index，这里尝试简单重建
        // 或者从 abstract 字段直接取（如果 API 提供了的话）
        let abstract_text = self.reconstruct_abstract(json);

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
            source: "openalex".to_string(),
        })
    }

    /// 从 OpenAlex 的 abstract_inverted_index 重建摘要文本
    ///
    /// OpenAlex 使用倒排索引格式存储摘要：
    /// { "word1": [0, 5], "word2": [1, 3], ... }
    /// 需要还原为按位置排列的完整文本。
    fn reconstruct_abstract(&self, json: &Value) -> Option<String> {
        let index = json["abstract_inverted_index"].as_object()?;
        if index.is_empty() {
            return None;
        }

        // 收集所有 (position, word) 对
        let mut words: Vec<(u64, &str)> = Vec::new();
        for (word, positions) in index {
            if let Some(pos_arr) = positions.as_array() {
                for pos in pos_arr {
                    if let Some(p) = pos.as_u64() {
                        words.push((p, word.as_str()));
                    }
                }
            }
        }

        // 按位置排序后拼接为完整文本
        words.sort_by_key(|(pos, _)| *pos);
        let text: String = words
            .iter()
            .map(|(_, w)| *w)
            .collect::<Vec<_>>()
            .join(" ");

        if text.is_empty() { None } else { Some(text) }
    }
}

impl PaperSource for OpenAlexClient {
    fn name(&self) -> &str {
        "openalex"
    }

    /// 按关键词搜索论文
    ///
    /// GET /works?search={q}&per_page={n}&mailto={email}&filter=...
    async fn search(
        &self,
        query: &str,
        limit: u32,
        year: Option<&str>,
        open_access_only: bool,
    ) -> Result<Vec<Paper>, AppError> {
        let url = format!("{BASE_URL}/works");

        // 构建过滤条件
        let mut filters = Vec::new();
        if let Some(y) = year {
            // 支持单一年份和年份范围
            if y.contains('-') {
                let parts: Vec<&str> = y.splitn(2, '-').collect();
                if parts.len() == 2 {
                    filters.push(format!("from_publication_date:{}-01-01", parts[0]));
                    filters.push(format!("to_publication_date:{}-12-31", parts[1]));
                }
            } else {
                filters.push(format!("from_publication_date:{y}-01-01"));
                filters.push(format!("to_publication_date:{y}-12-31"));
            }
        }
        if open_access_only {
            filters.push("is_oa:true".to_string());
        }

        let mut params: Vec<(&str, String)> = vec![
            ("search", query.to_string()),
            ("per_page", limit.to_string()),
            ("mailto", self.email.clone()),
        ];
        if !filters.is_empty() {
            params.push(("filter", filters.join(",")));
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
                AppError::NetworkError(format!("OpenAlex search request failed: {e}"))
            })?;

        let json = self.handle_response(response, "").await?;

        // 从 results 数组解析论文列表
        let papers = json["results"]
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
    /// GET /works/doi:{doi}?mailto={email}
    /// 注意: OpenAlex 使用小写 "doi:" 前缀
    async fn lookup(&self, doi: &str) -> Result<Paper, AppError> {
        let url = format!("{BASE_URL}/works/doi:{doi}");
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
                AppError::NetworkError(format!("OpenAlex lookup request failed: {e}"))
            })?;

        let json = self.handle_response(response, doi).await?;
        self.parse_paper(&json)
    }

    /// 获取论文的开放获取 PDF URL
    async fn get_pdf_url(&self, doi: &str) -> Result<Option<String>, AppError> {
        let paper = self.lookup(doi).await?;
        Ok(paper.best_oa_url)
    }
}
