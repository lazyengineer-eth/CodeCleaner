use dialoguer::{Confirm, MultiSelect, Select};

/// User action after reviewing the fix report.
pub enum FixAction {
    ApproveAll,
    SelectFixes(Vec<usize>),
    Cancel,
}

/// Prompt the user to choose what to do with the fixes.
pub fn prompt_fix_action(fix_count: usize) -> FixAction {
    let options = vec![
        format!("Approve all {fix_count} fixes and commit"),
        "Select specific fixes to keep".to_string(),
        "Cancel — revert all changes".to_string(),
    ];

    let selection = Select::new()
        .with_prompt("What would you like to do?")
        .items(&options)
        .default(0)
        .interact()
        .unwrap_or(2); // Default to cancel on error

    match selection {
        0 => FixAction::ApproveAll,
        1 => {
            // Let user pick which fixes to keep
            FixAction::SelectFixes(vec![]) // Will be filled by caller
        }
        _ => FixAction::Cancel,
    }
}

/// Let the user select which fixes to keep.
pub fn select_fixes(fix_descriptions: &[String]) -> Vec<usize> {
    MultiSelect::new()
        .with_prompt("Select fixes to keep (space to toggle, enter to confirm)")
        .items(fix_descriptions)
        .interact()
        .unwrap_or_default()
}

/// Simple yes/no confirmation.
pub fn confirm(message: &str) -> bool {
    Confirm::new()
        .with_prompt(message)
        .default(false)
        .interact()
        .unwrap_or(false)
}
