use globset::{Glob, GlobMatcher};
use regex::Regex;
use tracing::warn;

use super::types::{LocalFinding, ReviewRule, RulePattern, RulesFile, SkipRule};

/// Compiled rule for efficient matching.
struct CompiledRule {
    rule: ReviewRule,
    matcher: RuleMatcher,
}

enum RuleMatcher {
    Regex(Regex),
    ContentContains(String),
    FilePath(GlobMatcher),
    FileExtension(Vec<String>),
}

/// The rules engine applies local rules to code before sending to AI.
pub struct RulesEngine {
    skip_globs: Vec<(GlobMatcher, String)>,
    compiled_rules: Vec<CompiledRule>,
}

impl RulesEngine {
    pub fn new(rules_file: &RulesFile) -> Self {
        let skip_globs = rules_file
            .skip
            .iter()
            .filter_map(|s| {
                Glob::new(&s.glob)
                    .ok()
                    .map(|g| (g.compile_matcher(), s.reason.clone()))
            })
            .collect();

        let compiled_rules = rules_file
            .rule
            .iter()
            .filter(|r| r.enabled)
            .filter_map(|r| {
                let matcher = match &r.pattern {
                    RulePattern::Regex { expression } => match Regex::new(expression) {
                        Ok(re) => RuleMatcher::Regex(re),
                        Err(e) => {
                            warn!(rule_id = %r.id, error = %e, "Invalid regex pattern, skipping rule");
                            return None;
                        }
                    },
                    RulePattern::ContentContains { text } => {
                        RuleMatcher::ContentContains(text.clone())
                    }
                    RulePattern::FilePath { glob } => match Glob::new(glob) {
                        Ok(g) => RuleMatcher::FilePath(g.compile_matcher()),
                        Err(e) => {
                            warn!(rule_id = %r.id, error = %e, "Invalid glob pattern, skipping rule");
                            return None;
                        }
                    },
                    RulePattern::FileExtension { extensions } => {
                        RuleMatcher::FileExtension(extensions.clone())
                    }
                };
                Some(CompiledRule {
                    rule: r.clone(),
                    matcher,
                })
            })
            .collect();

        Self {
            skip_globs,
            compiled_rules,
        }
    }

    /// Check if a file should be skipped entirely.
    pub fn should_skip(&self, file_path: &str) -> Option<&str> {
        for (glob, reason) in &self.skip_globs {
            if glob.is_match(file_path) {
                return Some(reason);
            }
        }
        None
    }

    /// Scan content for rule matches.
    pub fn scan(&self, file_path: &str, content: &str) -> Vec<LocalFinding> {
        let mut findings = Vec::new();

        for compiled in &self.compiled_rules {
            match &compiled.matcher {
                RuleMatcher::Regex(re) => {
                    for (line_num, line) in content.lines().enumerate() {
                        if let Some(m) = re.find(line) {
                            findings.push(LocalFinding {
                                rule_id: compiled.rule.id.clone(),
                                rule_name: compiled.rule.name.clone(),
                                file_path: file_path.to_string(),
                                line: Some((line_num + 1) as u32),
                                severity: compiled.rule.severity.clone(),
                                message: compiled
                                    .rule
                                    .message_template
                                    .replace("{match}", m.as_str()),
                            });
                        }
                    }
                }
                RuleMatcher::ContentContains(text) => {
                    for (line_num, line) in content.lines().enumerate() {
                        if line.contains(text.as_str()) {
                            findings.push(LocalFinding {
                                rule_id: compiled.rule.id.clone(),
                                rule_name: compiled.rule.name.clone(),
                                file_path: file_path.to_string(),
                                line: Some((line_num + 1) as u32),
                                severity: compiled.rule.severity.clone(),
                                message: compiled.rule.message_template.clone(),
                            });
                        }
                    }
                }
                RuleMatcher::FilePath(glob) => {
                    if glob.is_match(file_path) {
                        findings.push(LocalFinding {
                            rule_id: compiled.rule.id.clone(),
                            rule_name: compiled.rule.name.clone(),
                            file_path: file_path.to_string(),
                            line: None,
                            severity: compiled.rule.severity.clone(),
                            message: compiled.rule.message_template.clone(),
                        });
                    }
                }
                RuleMatcher::FileExtension(extensions) => {
                    if let Some(ext) = file_path.rsplit('.').next() {
                        if extensions.iter().any(|e| e == ext) {
                            findings.push(LocalFinding {
                                rule_id: compiled.rule.id.clone(),
                                rule_name: compiled.rule.name.clone(),
                                file_path: file_path.to_string(),
                                line: None,
                                severity: compiled.rule.severity.clone(),
                                message: compiled.rule.message_template.clone(),
                            });
                        }
                    }
                }
            }
        }

        findings
    }

    /// Get all enabled rules (for prompt context).
    pub fn enabled_rules(&self) -> Vec<&ReviewRule> {
        self.compiled_rules.iter().map(|c| &c.rule).collect()
    }
}
