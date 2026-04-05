use crate::azure::types::ExistingComment;
use crate::gemini::types::AiReviewComment;

/// Deduplicate AI comments against existing PR comments.
/// Removes AI comments that are substantially similar to existing ones.
pub fn deduplicate(
    ai_comments: Vec<AiReviewComment>,
    existing: &[ExistingComment],
) -> Vec<AiReviewComment> {
    ai_comments
        .into_iter()
        .filter(|ai| {
            !existing.iter().any(|ex| {
                // Same file and similar line
                let same_file = ex
                    .file_path
                    .as_ref()
                    .map(|f| f.trim_start_matches('/') == ai.file_path.trim_start_matches('/'))
                    .unwrap_or(false);

                let same_line = ex
                    .line
                    .map(|l| (l as i64 - ai.line_number as i64).unsigned_abs() <= 3)
                    .unwrap_or(false);

                let similar_content = text_similarity(&ai.comment, &ex.content) > 0.5;

                same_file && (same_line || similar_content)
            })
        })
        .collect()
}

/// Simple word-based Jaccard similarity.
fn text_similarity(a: &str, b: &str) -> f64 {
    let words_a: std::collections::HashSet<_> = a.to_lowercase().split_whitespace().collect();
    let words_b: std::collections::HashSet<_> = b.to_lowercase().split_whitespace().collect();

    if words_a.is_empty() && words_b.is_empty() {
        return 1.0;
    }

    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}
