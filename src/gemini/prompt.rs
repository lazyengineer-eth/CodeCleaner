use crate::azure::types::ExistingComment;
use crate::rules::types::ReviewRule;

/// Build the AI review prompt for code changes.
pub fn build_review_prompt(
    pr_title: &str,
    pr_description: Option<&str>,
    repo_name: &str,
    source_branch: &str,
    target_branch: &str,
    diff_content: &str,
    existing_comments: &[ExistingComment],
    rules: &[ReviewRule],
) -> String {
    let mut prompt = String::with_capacity(diff_content.len() + 2048);

    prompt.push_str(
        "You are an expert code reviewer. Review the following code changes from a pull request.\n\n",
    );

    // PR Context
    prompt.push_str("## PR Context\n");
    prompt.push_str(&format!("- Title: {pr_title}\n"));
    if let Some(desc) = pr_description {
        if !desc.is_empty() {
            prompt.push_str(&format!("- Description: {desc}\n"));
        }
    }
    prompt.push_str(&format!("- Repository: {repo_name}\n"));
    prompt.push_str(&format!("- Source -> Target: {source_branch} -> {target_branch}\n\n"));

    // Existing comments
    if !existing_comments.is_empty() {
        prompt.push_str("## Existing Review Comments (DO NOT duplicate these)\n");
        for c in existing_comments {
            let location = match (&c.file_path, c.line) {
                (Some(f), Some(l)) => format!("{f}:{l}"),
                (Some(f), None) => f.clone(),
                _ => "general".into(),
            };
            prompt.push_str(&format!("- [{location}] {}: {}\n", c.author, c.content));
        }
        prompt.push('\n');
    }

    // Rules context
    let enabled_rules: Vec<_> = rules.iter().filter(|r| r.enabled).collect();
    if !enabled_rules.is_empty() {
        prompt.push_str("## Custom Review Rules to Watch For\n");
        for rule in &enabled_rules {
            prompt.push_str(&format!(
                "- [{}] {}: {}\n",
                rule.severity, rule.name, rule.message_template
            ));
        }
        prompt.push('\n');
    }

    // Code changes
    prompt.push_str("## Code Changes\n```diff\n");
    prompt.push_str(diff_content);
    prompt.push_str("\n```\n\n");

    // Instructions
    prompt.push_str(
        r#"## Instructions
1. Focus on bugs, security issues, performance problems, and maintainability.
2. Do NOT comment on formatting or style unless it causes a real problem.
3. Reference specific line numbers from the NEW file (lines starting with +).
4. For each issue, provide the EXACT line number in the new file where the issue occurs.
5. Do NOT repeat any existing review comments listed above.

Respond with a JSON array. Each element must have:
- "file_path": string (exact file path from the diff header)
- "line_number": integer (line number in the new version of the file)
- "severity": "error" | "warning" | "suggestion" | "nitpick"
- "category": string (e.g. "security", "performance", "logic", "error-handling")
- "comment": string (clear explanation of the issue)
- "suggested_fix": string or null (code suggestion if applicable)

If no issues are found, respond with an empty array: []
"#,
    );

    prompt
}

/// Build the AI fix prompt for a review comment.
pub fn build_fix_prompt(
    pr_title: &str,
    file_path: &str,
    reviewer_name: &str,
    line_number: Option<u32>,
    comment_text: &str,
    file_content: &str,
    language: &str,
) -> String {
    let mut prompt = String::with_capacity(file_content.len() + 1024);

    prompt.push_str(
        "You are an expert software engineer. A code review comment was left on a pull request.\n\
         Analyze whether the review is valid and if so, provide a fix.\n\n",
    );

    prompt.push_str("## PR Context\n");
    prompt.push_str(&format!("- Title: {pr_title}\n"));
    prompt.push_str(&format!("- File: {file_path}\n\n"));

    prompt.push_str("## Review Comment\n");
    prompt.push_str(&format!("Reviewer: {reviewer_name}\n"));
    if let Some(line) = line_number {
        prompt.push_str(&format!("Line: {line}\n"));
    }
    prompt.push_str(&format!("Comment: \"{comment_text}\"\n\n"));

    // File content with line numbers
    prompt.push_str(&format!("## Full File Content\n```{language}\n"));
    for (i, line) in file_content.lines().enumerate() {
        prompt.push_str(&format!("{:>4} | {line}\n", i + 1));
    }
    prompt.push_str("```\n\n");

    prompt.push_str(
        r#"Respond with JSON:
{
  "is_valid": true or false,
  "validity_reasoning": "why this review is valid or invalid — be specific",
  "fix": {
    "start_line": <first line number to replace (1-indexed)>,
    "end_line": <last line number to replace (1-indexed)>,
    "old_code": "exact original code being replaced",
    "new_code": "fixed code",
    "explanation": "what this fix does and why",
    "effect": "how this affects surrounding code and overall behavior"
  },
  "category": "security|performance|logic|error-handling|style|other"
}

If the review is NOT valid, set "fix" to null and explain why in "validity_reasoning".
"#,
    );

    prompt
}

/// Guess the programming language from a file extension.
pub fn detect_language(file_path: &str) -> &str {
    match file_path.rsplit('.').next() {
        Some("rs") => "rust",
        Some("cs") => "csharp",
        Some("js") => "javascript",
        Some("ts") => "typescript",
        Some("tsx") => "typescript",
        Some("jsx") => "javascript",
        Some("py") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("kt") => "kotlin",
        Some("rb") => "ruby",
        Some("cpp" | "cc" | "cxx") => "cpp",
        Some("c") => "c",
        Some("h" | "hpp") => "cpp",
        Some("swift") => "swift",
        Some("yaml" | "yml") => "yaml",
        Some("json") => "json",
        Some("xml") => "xml",
        Some("sql") => "sql",
        Some("sh" | "bash") => "bash",
        Some("ps1") => "powershell",
        _ => "",
    }
}
