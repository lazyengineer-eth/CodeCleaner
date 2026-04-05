use std::collections::HashMap;
use std::path::PathBuf;

use crate::azure::client::AzureClient;
use crate::azure::comments::{extract_existing_comments, filter_active_comments};
use crate::azure::types::PullRequest;
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::fix::analyzer::{analyze_comments, AnalyzedComment};
use crate::fix::git;
use crate::fix::patcher;
use crate::fix::report::FixReport;
use crate::ui::progress::create_spinner;
use crate::ui::prompt::{self, FixAction};
use crate::ui::report::print_fix_summary;
use tracing::{error, info, warn};

/// Run the fix workflow for a single PR.
pub async fn run_fix(
    config: &AppConfig,
    azure: &AzureClient,
    gemini: &crate::gemini::client::GeminiClient,
    pr: &PullRequest,
) -> AppResult<()> {
    let working_dir = std::env::current_dir()?;

    // 1. Verify local branch matches PR source branch
    let pr_branch = git::ref_to_branch_name(&pr.source_ref_name);
    let local_branch = git::current_branch(&working_dir)?;
    if local_branch != pr_branch {
        warn!(
            expected = %pr_branch,
            actual = %local_branch,
            "Local branch does not match PR branch"
        );
        return Err(AppError::BranchMismatch {
            expected: pr_branch.to_string(),
            actual: local_branch,
        });
    }

    info!(
        pr_id = pr.pull_request_id,
        branch = %pr_branch,
        "Starting fix mode"
    );

    // 2. Fetch all comment threads
    let spinner = create_spinner("Fetching review comments...");
    let threads = azure.get_comment_threads(pr.pull_request_id).await?;
    let all_comments = extract_existing_comments(&threads);
    let active_comments = filter_active_comments(&all_comments);
    spinner.finish_with_message(format!(
        "Found {} active review comments",
        active_comments.len()
    ));

    if active_comments.is_empty() {
        return Err(AppError::NoActiveComments(pr.pull_request_id));
    }

    // 3. Read local file contents for commented files
    let spinner = create_spinner("Reading local files...");
    let mut file_contents: HashMap<String, String> = HashMap::new();
    for comment in &active_comments {
        if let Some(ref path) = comment.file_path {
            let clean_path = path.trim_start_matches('/');
            if file_contents.contains_key(clean_path) {
                continue;
            }
            let local_path = working_dir.join(clean_path);
            match std::fs::read_to_string(&local_path) {
                Ok(content) => {
                    file_contents.insert(clean_path.to_string(), content);
                }
                Err(e) => {
                    warn!(file = %clean_path, error = %e, "Could not read local file");
                }
            }
        }
    }
    spinner.finish_with_message(format!("Read {} local files", file_contents.len()));

    // 4. Analyze each comment with Gemini
    let spinner = create_spinner("Analyzing review comments with AI...");
    let analyzed = analyze_comments(gemini, &pr.title, &active_comments, &file_contents).await?;
    spinner.finish_with_message("Analysis complete");

    let valid_fixes: Vec<&AnalyzedComment> = analyzed.iter().filter(|a| a.analysis.is_valid).collect();
    if valid_fixes.is_empty() {
        println!("\nNo valid fixes found. All review comments were determined to be invalid.");
        let report = FixReport {
            pr: pr.clone(),
            analyzed: analyzed.clone(),
        };
        println!("{}", report.render());
        return Ok(());
    }

    // 5. Create backups and apply fixes
    let backup_dir = working_dir.join(&config.fix.backup_dir);
    if config.fix.create_backup {
        for analyzed_item in &analyzed {
            if analyzed_item.analysis.is_valid {
                if let Some(ref fix) = analyzed_item.analysis.fix {
                    if let Some(ref path) = analyzed_item.comment.file_path {
                        let clean_path = path.trim_start_matches('/');
                        let local_path = working_dir.join(clean_path);
                        if local_path.exists() {
                            patcher::backup_file(&local_path, &backup_dir)?;
                        }
                    }
                }
            }
        }
    }

    // Apply fixes grouped by file
    let mut fixes_by_file: HashMap<String, Vec<&crate::gemini::types::CodeFix>> = HashMap::new();
    for analyzed_item in &analyzed {
        if let (true, Some(ref fix), Some(ref path)) = (
            analyzed_item.analysis.is_valid,
            &analyzed_item.analysis.fix,
            &analyzed_item.comment.file_path,
        ) {
            let clean_path = path.trim_start_matches('/').to_string();
            fixes_by_file.entry(clean_path).or_default().push(fix);
        }
    }

    for (file, mut fixes) in fixes_by_file {
        let local_path = working_dir.join(&file);
        if let Err(e) = patcher::apply_fixes(&local_path, &mut fixes) {
            error!(file = %file, error = %e, "Failed to apply fix");
        }
    }

    // 6. Show the report
    let report = FixReport {
        pr: pr.clone(),
        analyzed: analyzed.clone(),
    };
    println!("{}", report.render());

    // 7. Prompt user for action
    let valid_count = analyzed.iter().filter(|a| a.analysis.is_valid).count();
    let action = prompt::prompt_fix_action(valid_count);

    match action {
        FixAction::ApproveAll => {
            // Stage and commit all fixed files
            let files: Vec<&str> = analyzed
                .iter()
                .filter(|a| a.analysis.is_valid && a.analysis.fix.is_some())
                .filter_map(|a| a.comment.file_path.as_deref())
                .map(|p| p.trim_start_matches('/'))
                .collect();

            git::stage_files(&working_dir, &files)?;

            let commit_msg = format!(
                "fix: apply {} AI-suggested fixes for PR #{}\n\nFixes applied by CodeCleaner based on review comments.",
                valid_count, pr.pull_request_id
            );
            let hash = git::commit(&working_dir, &commit_msg)?;

            patcher::cleanup_backups(&backup_dir)?;
            print_fix_summary(
                pr.pull_request_id,
                valid_count,
                analyzed.len() - valid_count,
                &hash,
            );
        }
        FixAction::SelectFixes(_) => {
            // Let user pick which fixes to keep
            let descriptions: Vec<String> = analyzed
                .iter()
                .filter(|a| a.analysis.is_valid && a.analysis.fix.is_some())
                .map(|a| {
                    let file = a.comment.file_path.as_deref().unwrap_or("unknown");
                    let line = a.comment.line.map(|l| format!(":{l}")).unwrap_or_default();
                    format!("{file}{line}: {}", a.comment.content)
                })
                .collect();

            let selected = prompt::select_fixes(&descriptions);

            if selected.is_empty() {
                // Revert everything
                patcher::restore_backups(&backup_dir, &working_dir)?;
                patcher::cleanup_backups(&backup_dir)?;
                println!("All changes reverted.");
                return Ok(());
            }

            // Revert unselected fixes
            let valid_items: Vec<&AnalyzedComment> = analyzed
                .iter()
                .filter(|a| a.analysis.is_valid && a.analysis.fix.is_some())
                .collect();

            for (i, item) in valid_items.iter().enumerate() {
                if !selected.contains(&i) {
                    if let Some(ref path) = item.comment.file_path {
                        let clean = path.trim_start_matches('/');
                        let _ = git::restore_file(&working_dir, clean);
                    }
                }
            }

            // Re-apply only selected fixes
            // (After reverting, the selected ones are already applied from step 5)

            let files: Vec<&str> = selected
                .iter()
                .filter_map(|&i| valid_items.get(i))
                .filter_map(|a| a.comment.file_path.as_deref())
                .map(|p| p.trim_start_matches('/'))
                .collect();

            git::stage_files(&working_dir, &files)?;

            let commit_msg = format!(
                "fix: apply {}/{} selected AI-suggested fixes for PR #{}",
                selected.len(),
                valid_count,
                pr.pull_request_id
            );
            let hash = git::commit(&working_dir, &commit_msg)?;

            patcher::cleanup_backups(&backup_dir)?;
            print_fix_summary(
                pr.pull_request_id,
                selected.len(),
                analyzed.len() - selected.len(),
                &hash,
            );
        }
        FixAction::Cancel => {
            // Restore all files
            let restored = patcher::restore_backups(&backup_dir, &working_dir)?;
            patcher::cleanup_backups(&backup_dir)?;
            println!("Cancelled. Restored {} files.", restored);
        }
    }

    Ok(())
}
