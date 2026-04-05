use crate::azure::types::ExistingComment;
use crate::error::AppResult;
use crate::gemini::client::GeminiClient;
use crate::gemini::prompt;
use crate::gemini::types::FixAnalysis;
use tracing::info;

/// Result of analyzing a single review comment.
#[derive(Debug, Clone)]
pub struct AnalyzedComment {
    pub comment: ExistingComment,
    pub analysis: FixAnalysis,
}

/// Analyze all active review comments using Gemini.
pub async fn analyze_comments(
    gemini: &GeminiClient,
    pr_title: &str,
    comments: &[ExistingComment],
    file_contents: &std::collections::HashMap<String, String>,
) -> AppResult<Vec<AnalyzedComment>> {
    let mut results = Vec::with_capacity(comments.len());

    for (i, comment) in comments.iter().enumerate() {
        let file_path = match &comment.file_path {
            Some(p) => p.trim_start_matches('/').to_string(),
            None => {
                info!(thread_id = comment.thread_id, "Skipping comment without file path");
                continue;
            }
        };

        let content = match file_contents.get(&file_path) {
            Some(c) => c,
            None => {
                info!(file = %file_path, "Skipping comment — file not found locally");
                continue;
            }
        };

        let language = prompt::detect_language(&file_path);
        let fix_prompt = prompt::build_fix_prompt(
            pr_title,
            &file_path,
            &comment.author,
            comment.line,
            &comment.content,
            content,
            language,
        );

        info!(
            progress = format!("{}/{}", i + 1, comments.len()),
            file = %file_path,
            "Analyzing review comment"
        );

        let analysis = gemini.analyze_fix(&fix_prompt).await?;

        results.push(AnalyzedComment {
            comment: comment.clone(),
            analysis,
        });
    }

    Ok(results)
}
