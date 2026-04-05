use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::transport::rate_limiter::RateLimiter;
use crate::transport::retry::with_retry;
use base64::Engine;
use reqwest::Client;

use super::types::*;

/// Azure DevOps REST API client.
pub struct AzureClient {
    client: Client,
    base_url: String,
    repo_id: String,
    api_version: String,
    rate_limiter: RateLimiter,
}

impl AzureClient {
    pub fn new(config: &AppConfig) -> AppResult<Self> {
        let pat = config.ado_pat()?;
        let auth = base64::engine::general_purpose::STANDARD.encode(format!(":{pat}"));

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Basic {auth}").parse().unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(AppError::Http)?;

        let org = config.azure_devops.organization.trim_end_matches('/');
        let project = &config.azure_devops.project;
        let base_url = format!("{org}/{project}/_apis/git/repositories/{}", config.azure_devops.repository);

        Ok(Self {
            client,
            base_url,
            repo_id: config.azure_devops.repository.clone(),
            api_version: config.azure_devops.api_version.clone(),
            rate_limiter: RateLimiter::new(config.performance.ado_rate_limit),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.base_url)
    }

    fn api_version_param(&self) -> (&str, &str) {
        ("api-version", &self.api_version)
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str, query: &[(&str, &str)]) -> AppResult<T> {
        self.rate_limiter.wait().await;

        let response = with_retry(|| async {
            self.client
                .get(url)
                .query(query)
                .send()
                .await
        })
        .await?;

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(AppError::AzureAuth);
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::AzureApi {
                status: status.as_u16(),
                message: body,
            });
        }

        Ok(response.json().await?)
    }

    /// Get a single pull request by ID.
    pub async fn get_pull_request(&self, pr_id: u64) -> AppResult<PullRequest> {
        let url = self.url(&format!("pullrequests/{pr_id}"));
        self.get_json(&url, &[self.api_version_param()]).await
    }

    /// List pull requests filtered by source branch.
    pub async fn list_pull_requests_by_branch(&self, branch: &str) -> AppResult<Vec<PullRequest>> {
        let ref_name = if branch.starts_with("refs/") {
            branch.to_string()
        } else {
            format!("refs/heads/{branch}")
        };

        let url = self.url("pullrequests");
        let api_ver = self.api_version.clone();
        let response: AzureListResponse<PullRequest> = self
            .get_json(
                &url,
                &[
                    ("api-version", &api_ver),
                    ("searchCriteria.status", "active"),
                    ("searchCriteria.sourceRefName", &ref_name),
                ],
            )
            .await?;

        Ok(response.value)
    }

    /// Get all comment threads on a PR.
    pub async fn get_comment_threads(&self, pr_id: u64) -> AppResult<Vec<CommentThread>> {
        let url = self.url(&format!("pullrequests/{pr_id}/threads"));
        let response: AzureListResponse<CommentThread> = self
            .get_json(&url, &[self.api_version_param()])
            .await?;
        Ok(response.value)
    }

    /// Get iterations for a PR.
    pub async fn get_iterations(&self, pr_id: u64) -> AppResult<Vec<Iteration>> {
        let url = self.url(&format!("pullrequests/{pr_id}/iterations"));
        let response: AzureListResponse<Iteration> = self
            .get_json(&url, &[self.api_version_param()])
            .await?;
        Ok(response.value)
    }

    /// Get changes for a specific iteration.
    pub async fn get_iteration_changes(&self, pr_id: u64, iteration_id: u32) -> AppResult<IterationChanges> {
        let url = self.url(&format!("pullrequests/{pr_id}/iterations/{iteration_id}/changes"));
        self.get_json(&url, &[self.api_version_param()]).await
    }

    /// Get the diff/content for a specific file in the PR.
    /// Uses the items endpoint to get file content at a specific commit.
    pub async fn get_file_content(&self, path: &str, version: &str, version_type: &str) -> AppResult<String> {
        self.rate_limiter.wait().await;

        let url = self.url("items");
        let api_ver = self.api_version.clone();
        let response = with_retry(|| async {
            self.client
                .get(&url)
                .query(&[
                    ("api-version", api_ver.as_str()),
                    ("path", path),
                    ("version", version),
                    ("versionType", version_type),
                ])
                .send()
                .await
        })
        .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::AzureApi {
                status: status.as_u16(),
                message: body,
            });
        }

        Ok(response.text().await?)
    }

    /// Post a new comment thread to a PR.
    pub async fn post_comment_thread(
        &self,
        pr_id: u64,
        thread: &NewCommentThread,
    ) -> AppResult<CommentThread> {
        self.rate_limiter.wait().await;

        let url = self.url(&format!("pullrequests/{pr_id}/threads"));
        let api_ver = self.api_version.clone();

        let response = with_retry(|| {
            let thread_clone = serde_json::to_value(thread).unwrap();
            async {
                self.client
                    .post(&url)
                    .query(&[("api-version", api_ver.as_str())])
                    .json(&thread_clone)
                    .send()
                    .await
            }
        })
        .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::AzureApi {
                status: status.as_u16(),
                message: body,
            });
        }

        Ok(response.json().await?)
    }

    /// Get the diff between source and target branch for the PR.
    pub async fn get_pr_diff(&self, pr_id: u64) -> AppResult<String> {
        self.rate_limiter.wait().await;

        let url = self.url(&format!("pullrequests/{pr_id}/diffs"));
        let api_ver = self.api_version.clone();

        let response = with_retry(|| async {
            self.client
                .get(&url)
                .query(&[("api-version", api_ver.as_str())])
                .send()
                .await
        })
        .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::AzureApi {
                status: status.as_u16(),
                message: body,
            });
        }

        Ok(response.text().await?)
    }

    pub fn repo_id(&self) -> &str {
        &self.repo_id
    }
}
