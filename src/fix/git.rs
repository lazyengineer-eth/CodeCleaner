use crate::error::{AppError, AppResult};
use std::path::Path;
use std::process::Command;
use tracing::info;

/// Get the current git branch name.
pub fn current_branch(working_dir: &Path) -> AppResult<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(working_dir)
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git: {e}")))?;

    if !output.status.success() {
        return Err(AppError::Git(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Stage specific files.
pub fn stage_files(working_dir: &Path, files: &[&str]) -> AppResult<()> {
    if files.is_empty() {
        return Ok(());
    }

    let mut cmd = Command::new("git");
    cmd.arg("add").current_dir(working_dir);
    for file in files {
        cmd.arg(file);
    }

    let output = cmd
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git add: {e}")))?;

    if !output.status.success() {
        return Err(AppError::Git(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    info!(files = files.len(), "Staged files");
    Ok(())
}

/// Create a commit with a message.
pub fn commit(working_dir: &Path, message: &str) -> AppResult<String> {
    let output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(working_dir)
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git commit: {e}")))?;

    if !output.status.success() {
        return Err(AppError::Git(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    // Get the commit hash
    let hash_output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(working_dir)
        .output()
        .map_err(|e| AppError::Git(format!("Failed to get commit hash: {e}")))?;

    let hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();
    info!(hash = %hash, "Created commit");
    Ok(hash)
}

/// Restore a file to its git HEAD state.
pub fn restore_file(working_dir: &Path, file: &str) -> AppResult<()> {
    let output = Command::new("git")
        .args(["checkout", "HEAD", "--", file])
        .current_dir(working_dir)
        .output()
        .map_err(|e| AppError::Git(format!("Failed to restore file: {e}")))?;

    if !output.status.success() {
        return Err(AppError::Git(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(())
}

/// Extract branch name from Azure DevOps ref format.
/// e.g., "refs/heads/feature/my-branch" -> "feature/my-branch"
pub fn ref_to_branch_name(ref_name: &str) -> &str {
    ref_name
        .strip_prefix("refs/heads/")
        .unwrap_or(ref_name)
}
