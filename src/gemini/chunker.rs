use crate::azure::diff::{format_as_unified_diff, FileDiff};

/// A chunk of diffs that fits within the Gemini context window.
#[derive(Debug)]
pub struct DiffChunk {
    pub diffs: Vec<FileDiff>,
    pub estimated_tokens: usize,
}

/// Estimate token count from text (rough: ~4 bytes per token for code).
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Split file diffs into chunks that fit within the token budget.
pub fn chunk_diffs(diffs: Vec<FileDiff>, max_tokens: usize) -> Vec<DiffChunk> {
    let mut chunks = Vec::new();
    let mut current_diffs: Vec<FileDiff> = Vec::new();
    let mut current_tokens: usize = 0;

    for diff in diffs {
        let diff_text = format_as_unified_diff(&diff);
        let tokens = estimate_tokens(&diff_text);

        // If a single file exceeds the budget, it gets its own chunk
        if tokens >= max_tokens {
            // Flush current chunk
            if !current_diffs.is_empty() {
                chunks.push(DiffChunk {
                    diffs: std::mem::take(&mut current_diffs),
                    estimated_tokens: current_tokens,
                });
                current_tokens = 0;
            }
            chunks.push(DiffChunk {
                diffs: vec![diff],
                estimated_tokens: tokens,
            });
            continue;
        }

        // If adding this diff would exceed the budget, flush
        if current_tokens + tokens > max_tokens && !current_diffs.is_empty() {
            chunks.push(DiffChunk {
                diffs: std::mem::take(&mut current_diffs),
                estimated_tokens: current_tokens,
            });
            current_tokens = 0;
        }

        current_tokens += tokens;
        current_diffs.push(diff);
    }

    // Flush remaining
    if !current_diffs.is_empty() {
        chunks.push(DiffChunk {
            diffs: current_diffs,
            estimated_tokens: current_tokens,
        });
    }

    chunks
}

/// Calculate the token budget for input based on model and config.
pub fn calculate_token_budget(context_budget_pct: u8) -> usize {
    // Gemini 2.0 Flash has 1M tokens context, but we use a conservative estimate
    // to account for system prompt overhead
    let model_context = 1_000_000usize;
    model_context * (context_budget_pct as usize) / 100
}

/// Split file content into chunks for fix mode (when file is too large).
pub fn chunk_file_content(content: &str, max_tokens: usize) -> Vec<String> {
    let total_tokens = estimate_tokens(content);
    if total_tokens <= max_tokens {
        return vec![content.to_string()];
    }

    // Split by lines, grouping into chunks
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_tokens = 0;

    for line in lines {
        let line_tokens = estimate_tokens(line) + 1; // +1 for newline
        if current_tokens + line_tokens > max_tokens && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
            current_tokens = 0;
        }
        current.push_str(line);
        current.push('\n');
        current_tokens += line_tokens;
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}
