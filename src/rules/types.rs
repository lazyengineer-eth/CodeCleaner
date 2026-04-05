use serde::{Deserialize, Serialize};

/// Top-level structure of rules.toml.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RulesFile {
    pub meta: RulesMeta,
    #[serde(default)]
    pub skip: Vec<SkipRule>,
    #[serde(default)]
    pub rule: Vec<ReviewRule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RulesMeta {
    pub version: String,
    pub last_updated: String,
    #[serde(default)]
    pub total_reviews: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SkipRule {
    pub glob: String,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReviewRule {
    pub id: String,
    pub name: String,
    pub category: String,
    pub severity: String,
    pub enabled: bool,
    pub source: String,
    pub confidence: f32,
    #[serde(default)]
    pub hit_count: u64,
    pub message_template: String,
    pub pattern: RulePattern,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_matched: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learned_from_pr: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learned_date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RulePattern {
    Regex { expression: String },
    #[serde(rename = "file_extension")]
    FileExtension { extensions: Vec<String> },
    #[serde(rename = "file_path")]
    FilePath { glob: String },
    #[serde(rename = "content_contains")]
    ContentContains { text: String },
}

/// A finding from a local rule match (not AI).
#[derive(Debug, Clone)]
pub struct LocalFinding {
    pub rule_id: String,
    pub rule_name: String,
    pub file_path: String,
    pub line: Option<u32>,
    pub severity: String,
    pub message: String,
}
