pub mod fix;
pub mod review;

use crate::azure::client::AzureClient;
use crate::azure::types::PullRequest;
use crate::cli::PrSelector;
use crate::error::{AppError, AppResult};

/// Resolve a PR from the user's selector (--pr or --branch).
pub async fn resolve_pr(azure: &AzureClient, selector: &PrSelector) -> AppResult<PullRequest> {
    if let Some(id) = selector.pr {
        azure.get_pull_request(id).await
    } else if let Some(ref branch) = selector.branch {
        let prs = azure.list_pull_requests_by_branch(branch).await?;
        match prs.len() {
            0 => Err(AppError::NoPrForBranch(branch.clone())),
            1 => Ok(prs.into_iter().next().unwrap()),
            n => Err(AppError::MultiplePrs(n, branch.clone())),
        }
    } else {
        Err(AppError::Config("Must specify --pr or --branch".into()))
    }
}
