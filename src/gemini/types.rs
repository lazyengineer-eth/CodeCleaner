use serde::{Deserialize, Serialize};

// --- Request types ---

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequest {
    pub contents: Vec<Content>,
    pub generation_config: GenerationConfig,
}

#[derive(Serialize, Debug)]
pub struct Content {
    pub parts: Vec<Part>,
    pub role: String,
}

#[derive(Serialize, Debug)]
pub struct Part {
    pub text: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    pub response_mime_type: String,
    pub temperature: f32,
    pub max_output_tokens: u32,
}

// --- Response types ---

#[derive(Deserialize, Debug)]
pub struct GeminiResponse {
    pub candidates: Option<Vec<Candidate>>,
    pub error: Option<GeminiError>,
}

#[derive(Deserialize, Debug)]
pub struct GeminiError {
    pub code: Option<u16>,
    pub message: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Option<CandidateContent>,
    pub finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CandidateContent {
    pub parts: Option<Vec<CandidatePart>>,
}

#[derive(Deserialize, Debug)]
pub struct CandidatePart {
    pub text: Option<String>,
}

// --- AI output types ---

/// A single code review comment from the AI.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AiReviewComment {
    pub file_path: String,
    pub line_number: u32,
    pub severity: String,
    pub category: String,
    pub comment: String,
    pub suggested_fix: Option<String>,
}

/// AI analysis of whether a review comment is valid and how to fix it.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FixAnalysis {
    pub is_valid: bool,
    pub validity_reasoning: String,
    pub fix: Option<CodeFix>,
    pub category: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CodeFix {
    pub start_line: u32,
    pub end_line: u32,
    pub old_code: String,
    pub new_code: String,
    pub explanation: String,
    pub effect: String,
}
