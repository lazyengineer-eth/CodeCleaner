use serde::{Deserialize, Serialize};

/// Wrapper for Azure DevOps paginated responses.
#[derive(Deserialize, Debug)]
pub struct AzureListResponse<T> {
    pub value: Vec<T>,
    pub count: Option<u64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub pull_request_id: u64,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub source_ref_name: String,
    pub target_ref_name: String,
    pub repository: RepositoryRef,
    pub created_by: IdentityRef,
    pub creation_date: String,
    pub is_draft: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryRef {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IdentityRef {
    pub display_name: String,
    pub unique_name: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Iteration {
    pub id: u32,
    pub created_date: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IterationChanges {
    pub change_entries: Vec<ChangeEntry>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChangeEntry {
    pub change_tracking_id: u32,
    pub change_type: String,
    pub item: Option<ChangeItem>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChangeItem {
    pub path: Option<String>,
}

// --- Comment thread types ---

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommentThread {
    pub id: u64,
    pub status: Option<String>,
    pub thread_context: Option<ThreadContext>,
    pub comments: Vec<Comment>,
    pub is_deleted: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ThreadContext {
    pub file_path: Option<String>,
    pub right_file_start: Option<LinePosition>,
    pub right_file_end: Option<LinePosition>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LinePosition {
    pub line: u32,
    pub offset: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: u64,
    pub content: String,
    pub author: IdentityRef,
    pub comment_type: Option<String>,
}

// --- Types for posting comments ---

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NewCommentThread {
    pub comments: Vec<NewComment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_context: Option<ThreadContext>,
    pub status: u8, // 1 = Active
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NewComment {
    pub parent_comment_id: u64,
    pub content: String,
    pub comment_type: u8, // 1 = Text
}

/// Parsed representation of an existing review comment.
#[derive(Debug, Clone)]
pub struct ExistingComment {
    pub thread_id: u64,
    pub author: String,
    pub content: String,
    pub file_path: Option<String>,
    pub line: Option<u32>,
    pub status: String,
}
