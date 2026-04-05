use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Azure DevOps API error (status {status}): {message}")]
    AzureApi { status: u16, message: String },

    #[error("Azure DevOps authentication failed — check your PAT")]
    AzureAuth,

    #[error("No active PR found for branch '{0}'")]
    NoPrForBranch(String),

    #[error("Multiple PRs ({0}) found for branch '{1}' — use --pr <id> instead")]
    MultiplePrs(usize, String),

    #[error("PR #{0} not found")]
    PrNotFound(u64),

    #[error("Gemini API error (status {status}): {message}")]
    GeminiApi { status: u16, message: String },

    #[error("Gemini response parse error: {0}")]
    GeminiParse(String),

    #[error("Rate limited by {service} — retry after {retry_after_secs}s")]
    RateLimited {
        service: String,
        retry_after_secs: u64,
    },

    #[error("Rule file error: {0}")]
    RuleFile(String),

    #[error("Diff too large for {file_path} ({size_bytes} bytes, max {max_bytes})")]
    DiffTooLarge {
        file_path: String,
        size_bytes: u64,
        max_bytes: u64,
    },

    #[error("Local branch mismatch: expected '{expected}', got '{actual}'")]
    BranchMismatch { expected: String, actual: String },

    #[error("No active review comments found on PR #{0}")]
    NoActiveComments(u64),

    #[error("Git operation failed: {0}")]
    Git(String),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    TomlDeserialize(#[from] toml::de::Error),
}

pub type AppResult<T> = Result<T, AppError>;
