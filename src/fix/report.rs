use crate::azure::types::PullRequest;
use colored::Colorize;

use super::analyzer::AnalyzedComment;

/// A complete fix report for display to the user.
pub struct FixReport {
    pub pr: PullRequest,
    pub analyzed: Vec<AnalyzedComment>,
}

impl FixReport {
    /// Render the full report to a string for terminal display.
    pub fn render(&self) -> String {
        let mut out = String::new();

        let valid_count = self.analyzed.iter().filter(|a| a.analysis.is_valid).count();
        let invalid_count = self.analyzed.len() - valid_count;

        // Header
        out.push_str(&format!(
            "\n{}\n",
            "╔══════════════════════════════════════════════════════════════╗"
                .bright_blue()
        ));
        out.push_str(&format!(
            "{} PR #{}: {}{}",
            "║".bright_blue(),
            self.pr.pull_request_id,
            truncate(&self.pr.title, 45),
            padding(45 - self.pr.title.len().min(45))
        ));
        out.push_str(&format!(" {}\n", "║".bright_blue()));

        let branch = self
            .pr
            .source_ref_name
            .strip_prefix("refs/heads/")
            .unwrap_or(&self.pr.source_ref_name);
        out.push_str(&format!(
            "{} Branch: {:<53} {}\n",
            "║".bright_blue(),
            branch,
            "║".bright_blue()
        ));
        out.push_str(&format!(
            "{} Review Comments: {} total, {} valid, {} invalid{} {}\n",
            "║".bright_blue(),
            self.analyzed.len(),
            valid_count.to_string().green(),
            invalid_count.to_string().yellow(),
            padding(
                30usize.saturating_sub(
                    self.analyzed.len().to_string().len()
                        + valid_count.to_string().len()
                        + invalid_count.to_string().len()
                )
            ),
            "║".bright_blue()
        ));
        out.push_str(&format!(
            "{}\n",
            "╚══════════════════════════════════════════════════════════════╝"
                .bright_blue()
        ));

        // Valid fixes
        let mut fix_num = 0;
        for analyzed in &self.analyzed {
            if analyzed.analysis.is_valid {
                fix_num += 1;
                out.push_str(&render_valid_fix(fix_num, valid_count, analyzed));
            }
        }

        // Invalid/skipped
        for analyzed in &self.analyzed {
            if !analyzed.analysis.is_valid {
                out.push_str(&render_skipped(analyzed));
            }
        }

        out
    }
}

fn render_valid_fix(num: usize, total: usize, analyzed: &AnalyzedComment) -> String {
    let mut out = String::new();
    let file_path = analyzed
        .comment
        .file_path
        .as_deref()
        .unwrap_or("unknown");
    let line = analyzed
        .comment
        .line
        .map(|l| format!(":{l}"))
        .unwrap_or_default();

    out.push_str(&format!(
        "\n{}\n",
        format!("── Fix {num} of {total} ──────────────────────────────────────────")
            .bright_white()
    ));
    out.push_str(&format!(
        "File: {}{}\n",
        file_path.bright_cyan(),
        line.bright_cyan()
    ));
    out.push_str(&format!(
        "Review Comment: \"{}\"\n",
        analyzed.comment.content.dimmed()
    ));
    out.push_str(&format!("Reviewer: {}\n", analyzed.comment.author));
    out.push_str(&format!("\n{} Review Valid: Yes\n", "✓".green().bold()));
    out.push_str(&format!(
        "\n{}: {}\n",
        "Reasoning".bold(),
        analyzed.analysis.validity_reasoning
    ));

    if let Some(ref fix) = analyzed.analysis.fix {
        out.push_str(&format!(
            "\n{}:\n",
            "Fix Applied".bold()
        ));
        out.push_str(&format!(
            "{}\n{}\n{}\n",
            "┌─ Before ──────────────────────────────────────────────┐".red(),
            format_code_block(&fix.old_code, "│").red(),
            "└───────────────────────────────────────────────────────┘".red(),
        ));
        out.push_str(&format!(
            "{}\n{}\n{}\n",
            "┌─ After ───────────────────────────────────────────────┐".green(),
            format_code_block(&fix.new_code, "│").green(),
            "└───────────────────────────────────────────────────────┘".green(),
        ));
        out.push_str(&format!("\n{}: {}\n", "Explanation".bold(), fix.explanation));
        out.push_str(&format!("{}: {}\n", "Effect".bold(), fix.effect));
    }

    out
}

fn render_skipped(analyzed: &AnalyzedComment) -> String {
    let mut out = String::new();
    let file_path = analyzed
        .comment
        .file_path
        .as_deref()
        .unwrap_or("unknown");

    out.push_str(&format!(
        "\n{}\n",
        "── Skipped (Invalid Review) ───────────────────────────────"
            .yellow()
    ));
    out.push_str(&format!("File: {}\n", file_path.bright_cyan()));
    out.push_str(&format!(
        "Review Comment: \"{}\"\n",
        analyzed.comment.content.dimmed()
    ));
    out.push_str(&format!("\n{} Review Valid: No\n", "✗".red().bold()));
    out.push_str(&format!(
        "\n{}: {}\n",
        "Reasoning".bold(),
        analyzed.analysis.validity_reasoning
    ));

    out
}

fn format_code_block(code: &str, prefix: &str) -> String {
    code.lines()
        .map(|line| format!("{prefix} {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

fn padding(n: usize) -> String {
    " ".repeat(n)
}
