use colored::Colorize;

/// Print the review mode summary.
pub fn print_review_summary(
    pr_id: u64,
    pr_title: &str,
    comments_posted: usize,
    local_findings: usize,
    rules_learned: usize,
    dry_run: bool,
) {
    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════".bright_blue()
    );
    println!(
        "  {} PR #{}: {}",
        if dry_run {
            "DRY RUN".yellow().bold()
        } else {
            "REVIEW COMPLETE".green().bold()
        },
        pr_id,
        pr_title
    );
    println!(
        "{}",
        "═══════════════════════════════════════════════".bright_blue()
    );

    if dry_run {
        println!("  Would post {} AI review comments", comments_posted.to_string().cyan());
    } else {
        println!("  Posted {} AI review comments", comments_posted.to_string().cyan());
    }
    println!(
        "  Found {} local rule matches",
        local_findings.to_string().yellow()
    );
    if rules_learned > 0 {
        println!(
            "  Learned {} new patterns",
            rules_learned.to_string().green()
        );
    }
    println!();
}

/// Print the fix mode summary after commit.
pub fn print_fix_summary(
    pr_id: u64,
    fixes_applied: usize,
    fixes_skipped: usize,
    commit_hash: &str,
) {
    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════".bright_green()
    );
    println!("  {} PR #{}", "FIX COMPLETE".green().bold(), pr_id);
    println!(
        "{}",
        "═══════════════════════════════════════════════".bright_green()
    );
    println!(
        "  Applied {} fixes, skipped {}",
        fixes_applied.to_string().green(),
        fixes_skipped.to_string().yellow()
    );
    println!("  Commit: {}", commit_hash.bright_cyan());
    println!();
}
