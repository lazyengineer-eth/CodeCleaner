use std::sync::Arc;

/// A parsed diff for a single file.
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub file_path: Arc<str>,
    pub change_type: ChangeType,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Add,
    Edit,
    Delete,
    Rename,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub enum DiffLine {
    Context(String),
    Added(String),
    Removed(String),
}

impl ChangeType {
    pub fn from_azure(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "add" => Self::Add,
            "delete" => Self::Delete,
            "rename" => Self::Rename,
            _ => Self::Edit,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Edit => "edit",
            Self::Delete => "delete",
            Self::Rename => "rename",
        }
    }
}

/// Parse a unified diff string into structured hunks.
pub fn parse_unified_diff(raw: &str) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<HunkBuilder> = None;

    for line in raw.lines() {
        if line.starts_with("@@") {
            // Flush previous hunk
            if let Some(builder) = current_hunk.take() {
                hunks.push(builder.build());
            }
            // Parse @@ -old_start,old_count +new_start,new_count @@
            if let Some(h) = parse_hunk_header(line) {
                current_hunk = Some(h);
            }
        } else if let Some(ref mut builder) = current_hunk {
            if let Some(rest) = line.strip_prefix('+') {
                builder.lines.push(DiffLine::Added(rest.to_string()));
            } else if let Some(rest) = line.strip_prefix('-') {
                builder.lines.push(DiffLine::Removed(rest.to_string()));
            } else if let Some(rest) = line.strip_prefix(' ') {
                builder.lines.push(DiffLine::Context(rest.to_string()));
            } else if line.is_empty() {
                builder.lines.push(DiffLine::Context(String::new()));
            }
            // Skip lines like "\ No newline at end of file"
        }
    }

    // Flush last hunk
    if let Some(builder) = current_hunk {
        hunks.push(builder.build());
    }

    hunks
}

struct HunkBuilder {
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
    lines: Vec<DiffLine>,
}

impl HunkBuilder {
    fn build(self) -> Hunk {
        Hunk {
            old_start: self.old_start,
            old_count: self.old_count,
            new_start: self.new_start,
            new_count: self.new_count,
            lines: self.lines,
        }
    }
}

fn parse_hunk_header(line: &str) -> Option<HunkBuilder> {
    // Format: @@ -old_start,old_count +new_start,new_count @@
    let line = line.trim_start_matches('@').trim_start();
    let line = line.split("@@").next()?;
    let parts: Vec<&str> = line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let (old_start, old_count) = parse_range(parts[0].trim_start_matches('-'))?;
    let (new_start, new_count) = parse_range(parts[1].trim_start_matches('+'))?;

    Some(HunkBuilder {
        old_start,
        old_count,
        new_start,
        new_count,
        lines: Vec::new(),
    })
}

fn parse_range(s: &str) -> Option<(u32, u32)> {
    if let Some((start, count)) = s.split_once(',') {
        Some((start.parse().ok()?, count.parse().ok()?))
    } else {
        Some((s.parse().ok()?, 1))
    }
}

/// Convert a FileDiff back to unified diff text for prompts.
pub fn format_as_unified_diff(diff: &FileDiff) -> String {
    let mut out = String::new();
    out.push_str(&format!("--- a/{}\n", diff.file_path));
    out.push_str(&format!("+++ b/{}\n", diff.file_path));

    for hunk in &diff.hunks {
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        ));
        for line in &hunk.lines {
            match line {
                DiffLine::Context(s) => {
                    out.push(' ');
                    out.push_str(s);
                    out.push('\n');
                }
                DiffLine::Added(s) => {
                    out.push('+');
                    out.push_str(s);
                    out.push('\n');
                }
                DiffLine::Removed(s) => {
                    out.push('-');
                    out.push_str(s);
                    out.push('\n');
                }
            }
        }
    }

    out
}
