use crate::error::{AppError, AppResult};
use crate::gemini::types::CodeFix;
use std::path::Path;
use tracing::info;

/// Apply a list of fixes to a file. Fixes are applied bottom-to-top
/// to preserve line number validity.
pub fn apply_fixes(file_path: &Path, fixes: &mut Vec<&CodeFix>) -> AppResult<()> {
    let content = std::fs::read_to_string(file_path)?;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();

    // Sort fixes by start_line descending so we apply from bottom to top
    fixes.sort_by(|a, b| b.start_line.cmp(&a.start_line));

    for fix in fixes {
        let start = (fix.start_line as usize).saturating_sub(1);
        let end = (fix.end_line as usize).min(lines.len());

        if start >= lines.len() {
            info!(
                file = %file_path.display(),
                start_line = fix.start_line,
                "Fix start line exceeds file length, skipping"
            );
            continue;
        }

        // Replace the lines
        let new_lines: Vec<String> = fix.new_code.lines().map(String::from).collect();
        lines.splice(start..end, new_lines);
    }

    // Write back
    let output = lines.join("\n");
    std::fs::write(file_path, if content.ends_with('\n') {
        format!("{output}\n")
    } else {
        output
    })?;

    Ok(())
}

/// Create a backup of a file before modifying it.
pub fn backup_file(file_path: &Path, backup_dir: &Path) -> AppResult<()> {
    std::fs::create_dir_all(backup_dir)?;

    let relative = file_path
        .file_name()
        .ok_or_else(|| AppError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid file path",
        )))?;

    // Use a unique name to avoid collisions
    let backup_name = format!(
        "{}.{}.bak",
        relative.to_string_lossy(),
        chrono::Utc::now().timestamp_millis()
    );
    let backup_path = backup_dir.join(backup_name);

    std::fs::copy(file_path, &backup_path)?;
    info!(
        original = %file_path.display(),
        backup = %backup_path.display(),
        "Created backup"
    );

    Ok(())
}

/// Restore all backup files from the backup directory.
pub fn restore_backups(backup_dir: &Path, working_dir: &Path) -> AppResult<u32> {
    if !backup_dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in std::fs::read_dir(backup_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Parse backup name: "filename.ext.timestamp.bak"
        if name_str.ends_with(".bak") {
            // Find the original filename by stripping ".timestamp.bak"
            if let Some(dot_pos) = name_str.rfind('.') {
                let without_bak = &name_str[..dot_pos];
                if let Some(dot_pos2) = without_bak.rfind('.') {
                    let original_name = &without_bak[..dot_pos2];
                    let original_path = working_dir.join(original_name);
                    std::fs::copy(entry.path(), &original_path)?;
                    count += 1;
                    info!(file = %original_path.display(), "Restored from backup");
                }
            }
        }
    }

    Ok(count)
}

/// Clean up the backup directory.
pub fn cleanup_backups(backup_dir: &Path) -> AppResult<()> {
    if backup_dir.exists() {
        std::fs::remove_dir_all(backup_dir)?;
        info!(dir = %backup_dir.display(), "Cleaned up backup directory");
    }
    Ok(())
}
