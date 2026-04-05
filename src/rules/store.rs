use crate::error::{AppError, AppResult};
use std::path::Path;

use super::types::{ReviewRule, RulesFile, RulesMeta, SkipRule};

/// Load rules from a TOML file.
pub fn load_rules(path: &Path) -> AppResult<RulesFile> {
    if !path.exists() {
        return Ok(default_rules());
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| AppError::RuleFile(format!("Failed to read rules file: {e}")))?;
    toml::from_str(&content).map_err(|e| AppError::RuleFile(format!("Failed to parse rules: {e}")))
}

/// Save rules to a TOML file.
pub fn save_rules(path: &Path, rules: &RulesFile) -> AppResult<()> {
    let content = toml::to_string_pretty(rules)
        .map_err(|e| AppError::RuleFile(format!("Failed to serialize rules: {e}")))?;
    std::fs::write(path, content)
        .map_err(|e| AppError::RuleFile(format!("Failed to write rules file: {e}")))?;
    Ok(())
}

/// Append a learned rule to the rules file.
pub fn append_rule(path: &Path, rule: ReviewRule) -> AppResult<()> {
    let mut rules = load_rules(path)?;
    // Check for duplicate ID
    if rules.rule.iter().any(|r| r.id == rule.id) {
        return Ok(()); // Already exists
    }
    rules.rule.push(rule);
    rules.meta.last_updated = chrono::Utc::now().to_rfc3339();
    save_rules(path, &rules)
}

/// Remove a rule by ID.
pub fn remove_rule(path: &Path, rule_id: &str) -> AppResult<bool> {
    let mut rules = load_rules(path)?;
    let before = rules.rule.len();
    rules.rule.retain(|r| r.id != rule_id);
    if rules.rule.len() == before {
        return Ok(false); // Not found
    }
    save_rules(path, &rules)?;
    Ok(true)
}

/// Create a default rules file with sensible defaults.
pub fn default_rules() -> RulesFile {
    use super::types::RulePattern;

    RulesFile {
        meta: RulesMeta {
            version: "1.0".into(),
            last_updated: chrono::Utc::now().to_rfc3339(),
            total_reviews: 0,
        },
        skip: vec![
            SkipRule {
                glob: "**/*.generated.cs".into(),
                reason: "Auto-generated code".into(),
            },
            SkipRule {
                glob: "**/node_modules/**".into(),
                reason: "Third-party dependencies".into(),
            },
            SkipRule {
                glob: "**/*.min.js".into(),
                reason: "Minified files".into(),
            },
            SkipRule {
                glob: "**/*.designer.cs".into(),
                reason: "Visual Studio designer files".into(),
            },
        ],
        rule: vec![
            ReviewRule {
                id: "sec-001".into(),
                name: "Hardcoded credentials".into(),
                category: "security".into(),
                severity: "error".into(),
                enabled: true,
                source: "manual".into(),
                confidence: 1.0,
                hit_count: 0,
                message_template: "Potential hardcoded credential detected: `{match}`".into(),
                pattern: RulePattern::Regex {
                    expression: r#"(?i)(password|secret|api_key|token)\s*=\s*"[^"]+""#.into(),
                },
                last_matched: None,
                learned_from_pr: None,
                learned_date: None,
            },
            ReviewRule {
                id: "sec-002".into(),
                name: "SQL injection risk".into(),
                category: "security".into(),
                severity: "error".into(),
                enabled: true,
                source: "manual".into(),
                confidence: 1.0,
                hit_count: 0,
                message_template: "Possible SQL injection — use parameterized queries".into(),
                pattern: RulePattern::Regex {
                    expression: r#"(?i)(execute|query)\s*\(\s*[f"'].*\{.*\}"#.into(),
                },
                last_matched: None,
                learned_from_pr: None,
                learned_date: None,
            },
            ReviewRule {
                id: "style-001".into(),
                name: "TODO without ticket".into(),
                category: "style".into(),
                severity: "suggestion".into(),
                enabled: true,
                source: "manual".into(),
                confidence: 1.0,
                hit_count: 0,
                message_template: "TODO comment without a ticket reference".into(),
                pattern: RulePattern::Regex {
                    expression: r"//\s*TODO(?!\s*[\[(])".into(),
                },
                last_matched: None,
                learned_from_pr: None,
                learned_date: None,
            },
        ],
    }
}
