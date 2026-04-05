use crate::gemini::types::AiReviewComment;

/// Format an AI review comment as a markdown string for Azure DevOps.
pub fn format_review_comment(comment: &AiReviewComment) -> String {
    let severity_badge = match comment.severity.to_lowercase().as_str() {
        "error" => "**[Error]**",
        "warning" => "**[Warning]**",
        "suggestion" => "**[Suggestion]**",
        "nitpick" => "**[Nitpick]**",
        _ => "**[Info]**",
    };

    let mut body = format!(
        "{} {}\n\n{}\n",
        severity_badge, comment.category, comment.comment
    );

    if let Some(ref fix) = comment.suggested_fix {
        if !fix.is_empty() {
            body.push_str(&format!(
                "\n<details>\n<summary>Suggested fix</summary>\n\n```\n{fix}\n```\n</details>\n"
            ));
        }
    }

    body.push_str("\n---\n*Posted by CodeCleaner (AI-assisted review)*");

    body
}

/// Format severity level for terminal display.
pub fn severity_color(severity: &str) -> &str {
    match severity.to_lowercase().as_str() {
        "error" => "red",
        "warning" => "yellow",
        "suggestion" => "cyan",
        "nitpick" => "white",
        _ => "white",
    }
}
