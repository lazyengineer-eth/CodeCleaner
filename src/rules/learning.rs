use crate::gemini::types::AiReviewComment;
use chrono::Utc;
use std::collections::HashMap;

use super::types::{ReviewRule, RulePattern};

/// Extract potential new rules from AI review comments.
/// Groups by category and looks for recurring patterns.
pub fn extract_patterns(
    comments: &[AiReviewComment],
    pr_id: u64,
    min_occurrences: usize,
) -> Vec<ReviewRule> {
    // Group comments by category
    let mut by_category: HashMap<String, Vec<&AiReviewComment>> = HashMap::new();
    for comment in comments {
        by_category
            .entry(comment.category.clone())
            .or_default()
            .push(comment);
    }

    let mut new_rules = Vec::new();

    for (category, group) in &by_category {
        if group.len() < min_occurrences {
            continue;
        }

        // Look for common keywords in comments
        let common_words = find_common_keywords(group);
        if common_words.is_empty() {
            continue;
        }

        let keyword = common_words.first().unwrap();
        let id = format!(
            "learned-{}-{}",
            category,
            Utc::now().timestamp_millis()
        );

        // Use the most common comment as the template
        let representative = group
            .iter()
            .max_by_key(|c| c.comment.len())
            .unwrap();

        new_rules.push(ReviewRule {
            id,
            name: format!("{category}: {keyword}"),
            category: category.clone(),
            severity: representative.severity.clone(),
            enabled: true,
            source: "learned".into(),
            confidence: 0.5,
            hit_count: group.len() as u64,
            message_template: representative.comment.clone(),
            pattern: RulePattern::ContentContains {
                text: keyword.clone(),
            },
            last_matched: Some(Utc::now().to_rfc3339()),
            learned_from_pr: Some(pr_id),
            learned_date: Some(Utc::now().to_rfc3339()),
        });
    }

    new_rules
}

/// Find common keywords across a group of review comments.
fn find_common_keywords(comments: &[&AiReviewComment]) -> Vec<String> {
    let stop_words = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
        "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "can",
        "shall", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into",
        "through", "during", "before", "after", "above", "below", "between", "and", "but", "or",
        "nor", "not", "so", "yet", "both", "either", "neither", "each", "every", "all", "any",
        "few", "more", "most", "other", "some", "such", "no", "only", "own", "same", "than",
        "too", "very", "just", "because", "if", "when", "while", "this", "that", "these",
        "those", "it", "its", "they", "them", "their", "we", "us", "our", "you", "your", "he",
        "him", "his", "she", "her",
    ];

    let mut word_counts: HashMap<String, usize> = HashMap::new();

    for comment in comments {
        let words: Vec<String> = comment
            .comment
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3 && !stop_words.contains(&w.as_ref()))
            .map(String::from)
            .collect();

        for word in words {
            *word_counts.entry(word).or_default() += 1;
        }
    }

    let mut sorted: Vec<_> = word_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.into_iter().take(3).map(|(w, _)| w).collect()
}

/// Update confidence for existing learned rules based on new matches.
pub fn update_confidence(rules: &mut [ReviewRule], matched_ids: &[String]) {
    for rule in rules.iter_mut() {
        if rule.source == "learned" && matched_ids.contains(&rule.id) {
            rule.confidence = (rule.confidence + 0.1).min(1.0);
            rule.hit_count += 1;
            rule.last_matched = Some(Utc::now().to_rfc3339());
        }
    }
}
