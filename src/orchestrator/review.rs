use std::sync::Arc;

use crate::azure::client::AzureClient;
use crate::azure::comments::{extract_existing_comments, filter_active_comments};
use crate::azure::diff::{format_as_unified_diff, parse_unified_diff, ChangeType, FileDiff};
use crate::azure::types::PullRequest;
use crate::config::AppConfig;
use crate::error::AppResult;
use crate::gemini::chunker::{calculate_token_budget, chunk_diffs};
use crate::gemini::client::GeminiClient;
use crate::gemini::prompt::build_review_prompt;
use crate::review::comment::deduplicate;
use crate::review::mapper::map_to_threads;
use crate::rules::engine::RulesEngine;
use crate::rules::learning;
use crate::rules::store;
use crate::rules::types::RulesFile;
use crate::ui::progress::{create_progress, create_spinner};
use crate::ui::report::print_review_summary;
use tracing::{error, info};

/// Run the review workflow for a single PR.
pub async fn run_review(
    config: &AppConfig,
    azure: &AzureClient,
    gemini: &GeminiClient,
    pr: &PullRequest,
    rules_file: &mut RulesFile,
    dry_run: bool,
) -> AppResult<()> {
    info!(
        pr_id = pr.pull_request_id,
        title = %pr.title,
        "Starting review"
    );

    // 1. Fetch existing comments
    let spinner = create_spinner("Fetching existing comments...");
    let threads = azure.get_comment_threads(pr.pull_request_id).await?;
    let existing_comments = extract_existing_comments(&threads);
    let active_comments = filter_active_comments(&existing_comments);
    spinner.finish_with_message(format!("Found {} existing comments", existing_comments.len()));

    // 2. Fetch iterations and changes
    let spinner = create_spinner("Fetching PR changes...");
    let iterations = azure.get_iterations(pr.pull_request_id).await?;
    let latest_iter = iterations
        .last()
        .ok_or_else(|| crate::error::AppError::Config("No iterations found".into()))?;

    let changes = azure
        .get_iteration_changes(pr.pull_request_id, latest_iter.id)
        .await?;
    spinner.finish_with_message(format!("Found {} changed files", changes.change_entries.len()));

    // 3. Build rules engine
    let rules_engine = RulesEngine::new(rules_file);

    // 4. Fetch diffs and apply local rules
    let mut file_diffs: Vec<FileDiff> = Vec::new();
    let mut local_findings = Vec::new();
    let pb = create_progress(changes.change_entries.len() as u64, "Fetching diffs...");

    for entry in &changes.change_entries {
        pb.inc(1);
        let file_path = match entry.item.as_ref().and_then(|i| i.path.as_ref()) {
            Some(p) => p.clone(),
            None => continue,
        };

        // Check skip rules
        if let Some(reason) = rules_engine.should_skip(&file_path) {
            info!(file = %file_path, reason, "Skipping file");
            continue;
        }

        let change_type = ChangeType::from_azure(&entry.change_type);
        if change_type == ChangeType::Delete {
            continue; // No point reviewing deleted files
        }

        // Fetch file content at source branch to build diff
        let branch = pr
            .source_ref_name
            .strip_prefix("refs/heads/")
            .unwrap_or(&pr.source_ref_name);

        match azure
            .get_file_content(&file_path, branch, "branch")
            .await
        {
            Ok(content) => {
                // Check size limit
                if content.len() as u64 > config.performance.max_diff_size_bytes {
                    info!(
                        file = %file_path,
                        size = content.len(),
                        "File too large, skipping"
                    );
                    continue;
                }

                // Scan with local rules
                let findings = rules_engine.scan(&file_path, &content);
                local_findings.extend(findings);

                // For review, we need the diff. Build a simple diff representation.
                let hunks = parse_unified_diff(&content);
                let diff = FileDiff {
                    file_path: Arc::from(file_path.as_str()),
                    change_type,
                    hunks,
                };
                file_diffs.push(diff);
            }
            Err(e) => {
                error!(file = %file_path, error = %e, "Failed to fetch file");
            }
        }
    }
    pb.finish_with_message("Diffs fetched");

    // 5. Chunk diffs for Gemini
    let token_budget = calculate_token_budget(config.gemini.context_budget_pct);
    let chunks = chunk_diffs(file_diffs, token_budget);
    info!(chunks = chunks.len(), "Split into chunks for AI review");

    // 6. Send to Gemini for review
    let mut all_ai_comments = Vec::new();
    let enabled_rules: Vec<_> = rules_engine.enabled_rules().into_iter().cloned().collect();
    let pb = create_progress(chunks.len() as u64, "AI reviewing...");

    for chunk in &chunks {
        pb.inc(1);

        // Build combined diff text for this chunk
        let diff_text: String = chunk
            .diffs
            .iter()
            .map(|d| format_as_unified_diff(d))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = build_review_prompt(
            &pr.title,
            pr.description.as_deref(),
            azure.repo_id(),
            &pr.source_ref_name,
            &pr.target_ref_name,
            &diff_text,
            &active_comments,
            &enabled_rules,
        );

        match gemini.review_code(&prompt).await {
            Ok(comments) => all_ai_comments.extend(comments),
            Err(e) => error!(error = %e, "Gemini review failed for chunk"),
        }
    }
    pb.finish_with_message("AI review complete");

    // 7. Deduplicate against existing comments
    let ai_comments = deduplicate(all_ai_comments, &existing_comments);

    // Limit to max_comments_per_pr
    let ai_comments: Vec<_> = ai_comments
        .into_iter()
        .take(config.review.max_comments_per_pr)
        .collect();

    info!(
        count = ai_comments.len(),
        "New review comments after deduplication"
    );

    // 8. Post comments (or print in dry-run)
    if !ai_comments.is_empty() {
        let threads_to_post = map_to_threads(&ai_comments);

        if dry_run {
            println!("\n--- DRY RUN: Would post these comments ---\n");
            for comment in &ai_comments {
                println!(
                    "  [{}] {}:{} - {}",
                    comment.severity, comment.file_path, comment.line_number, comment.comment
                );
                if let Some(ref fix) = comment.suggested_fix {
                    println!("    Suggested: {fix}");
                }
                println!();
            }
        } else {
            let pb = create_progress(threads_to_post.len() as u64, "Posting comments...");
            for thread in &threads_to_post {
                pb.inc(1);
                if let Err(e) = azure
                    .post_comment_thread(pr.pull_request_id, thread)
                    .await
                {
                    error!(error = %e, "Failed to post comment thread");
                }
            }
            pb.finish_with_message("Comments posted");
        }
    }

    // 9. Learn patterns
    let mut rules_learned = 0;
    if config.rules.auto_learn {
        let new_rules = learning::extract_patterns(&ai_comments, pr.pull_request_id, 3);
        for rule in new_rules {
            if let Err(e) = store::append_rule(&config.rules.file, rule) {
                error!(error = %e, "Failed to save learned rule");
            } else {
                rules_learned += 1;
            }
        }
        rules_file.meta.total_reviews += 1;
        let _ = store::save_rules(&config.rules.file, rules_file);
    }

    // 10. Print summary
    print_review_summary(
        pr.pull_request_id,
        &pr.title,
        ai_comments.len(),
        local_findings.len(),
        rules_learned,
        dry_run,
    );

    Ok(())
}
