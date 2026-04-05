use crate::error::{AppError, AppResult};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub azure_devops: AzureDevOpsConfig,
    pub gemini: GeminiConfig,
    pub review: ReviewConfig,
    pub fix: FixConfig,
    pub performance: PerformanceConfig,
    pub rules: RulesConfig,
    pub logging: LoggingConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AzureDevOpsConfig {
    pub organization: String,
    pub project: String,
    /// Environment variable name holding the PAT
    #[serde(default = "default_pat_env")]
    pub pat_env_var: String,
    #[serde(default = "default_api_version")]
    pub api_version: String,
    pub repository: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GeminiConfig {
    /// Environment variable name holding the API key
    #[serde(default = "default_gemini_env")]
    pub api_key_env_var: String,
    #[serde(default = "default_gemini_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u32,
    #[serde(default = "default_context_budget")]
    pub context_budget_pct: u8,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ReviewConfig {
    #[serde(default = "default_min_severity")]
    pub min_severity: String,
    #[serde(default = "default_max_comments")]
    pub max_comments_per_pr: usize,
    #[serde(default = "default_true")]
    pub include_suggestions: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FixConfig {
    #[serde(default = "default_true")]
    pub create_backup: bool,
    #[serde(default = "default_backup_dir")]
    pub backup_dir: String,
    #[serde(default = "default_true")]
    pub auto_stage: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PerformanceConfig {
    #[serde(default = "default_ado_rate")]
    pub ado_rate_limit: u32,
    #[serde(default = "default_gemini_rate")]
    pub gemini_rate_limit_rpm: u32,
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_secs: u64,
    #[serde(default = "default_max_diff")]
    pub max_diff_size_bytes: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RulesConfig {
    #[serde(default = "default_rules_file")]
    pub file: PathBuf,
    #[serde(default = "default_true")]
    pub auto_learn: bool,
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_file")]
    pub file: String,
    #[serde(default)]
    pub log_to_file: bool,
}

// Defaults
fn default_pat_env() -> String { "CODECLEANER_ADO_PAT".into() }
fn default_gemini_env() -> String { "CODECLEANER_GEMINI_KEY".into() }
fn default_api_version() -> String { "7.1".into() }
fn default_gemini_model() -> String { "gemini-2.0-flash".into() }
fn default_temperature() -> f32 { 0.2 }
fn default_max_output_tokens() -> u32 { 8192 }
fn default_context_budget() -> u8 { 75 }
fn default_min_severity() -> String { "suggestion".into() }
fn default_max_comments() -> usize { 25 }
fn default_true() -> bool { true }
fn default_backup_dir() -> String { ".codecleaner_backup".into() }
fn default_ado_rate() -> u32 { 10 }
fn default_gemini_rate() -> u32 { 60 }
fn default_cache_ttl() -> u64 { 300 }
fn default_max_diff() -> u64 { 524_288 }
fn default_rules_file() -> PathBuf { PathBuf::from("rules.toml") }
fn default_min_confidence() -> f32 { 0.7 }
fn default_log_level() -> String { "info".into() }
fn default_log_file() -> String { "codecleaner.log".into() }

impl AppConfig {
    pub fn load(path: &Path) -> AppResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            AppError::Config(format!("Failed to read config file '{}': {e}", path.display()))
        })?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> AppResult<()> {
        // Verify required env vars exist
        if std::env::var(&self.azure_devops.pat_env_var).is_err() {
            return Err(AppError::Config(format!(
                "Environment variable '{}' not set — required for Azure DevOps PAT",
                self.azure_devops.pat_env_var
            )));
        }
        if std::env::var(&self.gemini.api_key_env_var).is_err() {
            return Err(AppError::Config(format!(
                "Environment variable '{}' not set — required for Gemini API key",
                self.gemini.api_key_env_var
            )));
        }
        if self.gemini.context_budget_pct > 95 {
            return Err(AppError::Config(
                "context_budget_pct must be <= 95 to leave room for output".into(),
            ));
        }
        Ok(())
    }

    pub fn ado_pat(&self) -> AppResult<String> {
        std::env::var(&self.azure_devops.pat_env_var).map_err(|_| {
            AppError::Config(format!("'{}' environment variable not set", self.azure_devops.pat_env_var))
        })
    }

    pub fn gemini_api_key(&self) -> AppResult<String> {
        std::env::var(&self.gemini.api_key_env_var).map_err(|_| {
            AppError::Config(format!("'{}' environment variable not set", self.gemini.api_key_env_var))
        })
    }
}
