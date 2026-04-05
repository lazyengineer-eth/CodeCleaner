use crate::azure::types::{LinePosition, NewComment, NewCommentThread, ThreadContext};
use crate::gemini::types::AiReviewComment;

use super::formatter::format_review_comment;

/// Map AI review comments to Azure DevOps comment threads.
pub fn map_to_threads(comments: &[AiReviewComment]) -> Vec<NewCommentThread> {
    comments
        .iter()
        .map(|comment| {
            let body = format_review_comment(comment);

            let thread_context = if comment.line_number > 0 {
                Some(ThreadContext {
                    file_path: Some(normalize_path(&comment.file_path)),
                    right_file_start: Some(LinePosition {
                        line: comment.line_number,
                        offset: 1,
                    }),
                    right_file_end: Some(LinePosition {
                        line: comment.line_number,
                        offset: 1,
                    }),
                })
            } else {
                None
            };

            NewCommentThread {
                comments: vec![NewComment {
                    parent_comment_id: 0,
                    content: body,
                    comment_type: 1, // Text
                }],
                thread_context,
                status: 1, // Active
            }
        })
        .collect()
}

/// Ensure path starts with / for Azure DevOps.
fn normalize_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}
