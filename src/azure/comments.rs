use super::types::{CommentThread, ExistingComment};

/// Extract structured existing comments from Azure DevOps threads.
pub fn extract_existing_comments(threads: &[CommentThread]) -> Vec<ExistingComment> {
    let mut result = Vec::new();

    for thread in threads {
        if thread.is_deleted.unwrap_or(false) {
            continue;
        }

        let status = thread.status.clone().unwrap_or_else(|| "unknown".into());
        let file_path = thread
            .thread_context
            .as_ref()
            .and_then(|ctx| ctx.file_path.clone());
        let line = thread
            .thread_context
            .as_ref()
            .and_then(|ctx| ctx.right_file_start.as_ref())
            .map(|pos| pos.line);

        for comment in &thread.comments {
            // Skip system-generated comments
            if comment.comment_type.as_deref() == Some("system") {
                continue;
            }

            result.push(ExistingComment {
                thread_id: thread.id,
                author: comment.author.display_name.clone(),
                content: comment.content.clone(),
                file_path: file_path.clone(),
                line,
                status: status.clone(),
            });
        }
    }

    result
}

/// Filter to only active/unresolved comments.
pub fn filter_active_comments(comments: &[ExistingComment]) -> Vec<ExistingComment> {
    comments
        .iter()
        .filter(|c| {
            let status = c.status.to_lowercase();
            status == "active" || status == "unknown" || status == "pending"
        })
        .cloned()
        .collect()
}
