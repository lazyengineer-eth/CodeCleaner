# CodeCleaner

AI-powered code review tool for Azure DevOps, written in Rust. Reviews pull request code changes using Google Gemini and posts intelligent review comments directly to your PRs. Goes beyond reviewing — it can also **auto-fix** existing review comments locally with full explanations.

---

## Table of Contents

- [Features](#features)
- [How It Works](#how-it-works)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
  - [Review Mode](#review-mode)
  - [Fix Mode](#fix-mode)
  - [Rules Management](#rules-management)
  - [Config Validation](#config-validation)
- [Review Rules & Auto-Learning](#review-rules--auto-learning)
- [Project Structure](#project-structure)
- [Performance & Memory Efficiency](#performance--memory-efficiency)
- [Comparison with CodeRabbit](#comparison-with-coderabbit)
- [License](#license)

---

## Features

- **AI Code Review** — Analyzes PR diffs using Google Gemini and posts inline review comments to Azure DevOps
- **Auto-Fix Mode** — Fetches existing review comments, validates them with AI, generates code fixes, applies them locally, and lets you approve before committing
- **Self-Learning Rules** — Maintains a local `rules.toml` that grows over time as the tool learns recurring patterns from AI reviews
- **Local-First** — Runs entirely on your machine. Your code never passes through third-party infrastructure (only diffs are sent to Gemini API)
- **Memory Efficient** — Streaming diffs, bounded concurrency, LRU caching, eager memory release
- **Rate Limited** — Built-in token-bucket rate limiting for both Azure DevOps and Gemini APIs with exponential backoff retry
- **Interactive Reports** — Rich terminal output with colored before/after diffs, severity badges, and fix explanations
- **Zero Subscription Cost** — You only pay for Gemini API usage (typically pennies per review)

---

## How It Works

### Review Mode

```
You run: codecleaner review --pr 1234

1. Fetches the PR and all existing review comments from Azure DevOps
2. Fetches code changes (diffs) for the PR
3. Applies local rules (rules.toml) for quick pattern matches
4. Sends diffs + context to Google Gemini for AI review
5. Deduplicates AI findings against existing comments (no repeats)
6. Posts new review comments as inline threads to the PR
7. Learns recurring patterns and updates rules.toml
```

### Fix Mode

```
You run: codecleaner fix --pr 1234

1. Fetches all active/unresolved review comments from the PR
2. Reads the full local source files for each commented file
3. For each review comment, AI analyzes:
   - Is this review valid? (with reasoning)
   - If valid: generates a concrete code fix
   - What the fix changes and how it affects surrounding code
4. Applies all valid fixes to your local files (with backup)
5. Presents a detailed interactive report:
   - Before/after code diffs for each fix
   - Validity reasoning for each comment
   - Skipped invalid reviews with explanations
6. You choose: Approve All / Select Specific Fixes / Cancel
7. On approval: stages and commits changes to your branch
```

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    CLI (clap)                                 │
│  codecleaner review --pr <id> | --branch <name>              │
│  codecleaner fix --pr <id> | --branch <name>                 │
│  codecleaner rules list | add | remove                       │
│  codecleaner config validate                                 │
└──────────────┬──────────────────────────┬────────────────────┘
               │                          │
       ┌───────▼────────┐        ┌────────▼─────────┐
       │ Review          │        │ Fix               │
       │ Orchestrator    │        │ Orchestrator      │
       │                 │        │                   │
       │ fetch PR        │        │ fetch PR          │
       │ fetch comments  │        │ fetch comments    │
       │ fetch diffs     │        │ fetch full files  │
       │ apply rules     │        │ validate reviews  │
       │ AI review       │        │ generate fixes    │
       │ post comments   │        │ apply locally     │
       └──┬────┬────┬───┘        │ present report    │
          │    │    │             │ user approves     │
          │    │    │             │ git commit        │
          │    │    │             └──┬────┬────┬──┬───┘
          │    │    │                │    │    │  │
  ┌───────▼┐ ┌▼────▼──┐    ┌───────▼┐ ┌▼────▼┐ ▼
  │Azure   │ │Gemini  │    │Azure   │ │Gemini│ ┌──────────┐
  │DevOps  │ │Client  │    │DevOps  │ │Client│ │Local Git │
  │Client  │ │        │    │Client  │ │      │ │& File Ops│
  └───┬────┘ └───┬────┘    └───┬────┘ └──┬───┘ └──────────┘
      │          │              │         │
  ┌───▼──────────▼──────────────▼─────────▼───┐
  │        Transport Layer (reqwest)           │
  │  Rate limiter + retry with backoff + cache │
  └────────────────────────────────────────────┘

  ┌────────────────────────────────────────────┐
  │         Rules Engine (rules.toml)          │
  │  Local pattern matching + auto-learning    │
  └────────────────────────────────────────────┘
```

---

## Prerequisites

- **Rust** (1.75 or later) — [Install Rust](https://rustup.rs/)
- **Azure DevOps account** with a [Personal Access Token (PAT)](https://learn.microsoft.com/en-us/azure/devops/organizations/accounts/use-personal-access-tokens-to-authenticate) that has **Code (Read & Write)** scope
- **Google Gemini API key** — [Get one here](https://aistudio.google.com/app/apikey)
- **Git** installed and configured

---

## Installation

### Build from source

```bash
git clone https://github.com/lazyengineer-eth/CodeCleaner.git
cd CodeCleaner
cargo build --release
```

The compiled binary will be at `target/release/codecleaner` (or `codecleaner.exe` on Windows).

### Add to PATH (optional)

**Windows (PowerShell):**
```powershell
Copy-Item target\release\codecleaner.exe C:\Users\<you>\bin\
# Add C:\Users\<you>\bin to your PATH environment variable
```

**macOS/Linux:**
```bash
cp target/release/codecleaner ~/.local/bin/
```

---

## Configuration

### 1. Create config file

```bash
cp config.example.toml config.toml
```

### 2. Edit `config.toml`

```toml
[azure_devops]
organization = "https://dev.azure.com/your-org"
project = "YourProject"
repository = "your-repo"
pat_env_var = "CODECLEANER_ADO_PAT"       # Env var name (not the token itself)
api_version = "7.1"

[gemini]
api_key_env_var = "CODECLEANER_GEMINI_KEY" # Env var name (not the key itself)
model = "gemini-2.0-flash"
temperature = 0.2
max_output_tokens = 8192
context_budget_pct = 75                    # % of context window to use for input

[review]
min_severity = "suggestion"                # error, warning, suggestion, nitpick
max_comments_per_pr = 25
include_suggestions = true

[fix]
create_backup = true
backup_dir = ".codecleaner_backup"
auto_stage = true

[performance]
ado_rate_limit = 10                        # Requests/sec to Azure DevOps
gemini_rate_limit_rpm = 60                 # Requests/min to Gemini
cache_ttl_secs = 300
max_diff_size_bytes = 524288               # 512 KB — skip larger files

[rules]
file = "rules.toml"
auto_learn = true
min_confidence = 0.7

[logging]
level = "info"                             # trace, debug, info, warn, error
file = "codecleaner.log"
log_to_file = false
```

### 3. Set environment variables

**Windows (Command Prompt):**
```cmd
set CODECLEANER_ADO_PAT=your-azure-devops-pat-here
set CODECLEANER_GEMINI_KEY=your-gemini-api-key-here
```

**Windows (PowerShell):**
```powershell
$env:CODECLEANER_ADO_PAT = "your-azure-devops-pat-here"
$env:CODECLEANER_GEMINI_KEY = "your-gemini-api-key-here"
```

**macOS/Linux:**
```bash
export CODECLEANER_ADO_PAT="your-azure-devops-pat-here"
export CODECLEANER_GEMINI_KEY="your-gemini-api-key-here"
```

> **Security:** Tokens are never stored in config files. They are always read from environment variables at runtime.

---

## Usage

### Review Mode

Review a PR's code changes and post AI comments to Azure DevOps.

```bash
# By PR ID
codecleaner review --pr 1234

# By branch name
codecleaner review --branch feature/add-auth

# Dry run — see what would be posted without actually posting
codecleaner review --pr 1234 --dry-run
```

**What happens:**
1. Fetches the PR, existing comments, and code diffs
2. Runs local rules first (fast pattern matching)
3. Sends diffs to Gemini for AI review
4. Deduplicates against existing comments
5. Posts new inline comments to Azure DevOps

**Example output:**
```
Reviewing PR #1234: Add payment processing endpoint

⠹ Fetching existing comments... Found 3 existing comments
⠹ Fetching PR changes... Found 8 changed files
[========================================] 8/8 Fetching diffs...
[========================================] 2/2 AI reviewing...

═══════════════════════════════════════════════
  REVIEW COMPLETE PR #1234: Add payment processing endpoint
═══════════════════════════════════════════════
  Posted 5 AI review comments
  Found 2 local rule matches
  Learned 1 new pattern
```

### Fix Mode

Fetch existing review comments on a PR, auto-fix them locally, and commit.

```bash
# By PR ID
codecleaner fix --pr 1234

# By branch name
codecleaner fix --branch feature/add-auth
```

> **Important:** Your local branch must match the PR's source branch. Check out the branch before running fix mode.

**What happens:**
1. Fetches all active/unresolved review comments
2. AI analyzes each comment for validity
3. Generates and applies fixes to local files
4. Shows an interactive report with full details
5. You choose what to commit

**Example report:**
```
╔══════════════════════════════════════════════════════════════╗
║  PR #1234: Add payment processing endpoint                  ║
║  Branch: feature/payment-api                                ║
║  Review Comments: 7 total, 5 valid, 2 invalid               ║
╚══════════════════════════════════════════════════════════════╝

── Fix 1 of 5 ─────────────────────────────────────────────
File: src/payment/handler.rs:42
Review Comment: "This unwrap() will panic on invalid input"
Reviewer: john.doe@company.com

✓ Review Valid: Yes

Reasoning: The unwrap() on line 42 is called on user input
from the HTTP request body. If the JSON is malformed, this
will panic and crash the server.

Fix Applied:
┌─ Before ──────────────────────────────────────────────┐
│ let amount = payload.get("amount").unwrap();          │
└───────────────────────────────────────────────────────┘
┌─ After ───────────────────────────────────────────────┐
│ let amount = payload.get("amount")                    │
│     .ok_or(AppError::MissingField("amount"))?;       │
└───────────────────────────────────────────────────────┘

Effect: Requests with missing "amount" field now return
HTTP 400 instead of crashing. No impact on valid requests.

── Skipped (Invalid Review) ───────────────────────────────
File: src/payment/models.rs:15
Review Comment: "This struct should derive Copy"

✗ Review Valid: No

Reasoning: The struct contains a String field which does
not implement Copy. The reviewer's suggestion is incorrect.

? What would you like to do?
> Approve all 5 fixes and commit
  Select specific fixes to keep
  Cancel — revert all changes
```

### Rules Management

```bash
# List all rules (manual + learned)
codecleaner rules list

# Remove a rule by ID
codecleaner rules remove learned-security-1712345678
```

### Config Validation

```bash
codecleaner config validate
```

Checks that your config file is valid and required environment variables are set.

---

## Review Rules & Auto-Learning

CodeCleaner uses a `rules.toml` file that combines manually defined rules with automatically learned patterns.

### Rule Types

| Pattern Type | Description | Example |
|---|---|---|
| `regex` | Regular expression match on file content | Detect hardcoded credentials |
| `content_contains` | Simple substring match | Find `Console.WriteLine` |
| `file_path` | Glob pattern on file paths | Flag changes to config files |
| `file_extension` | Match by file extension | Skip `.min.js` files |

### Skip Rules

Files matching skip globs are excluded from review entirely:

```toml
[[skip]]
glob = "**/*.generated.cs"
reason = "Auto-generated code"

[[skip]]
glob = "**/node_modules/**"
reason = "Third-party dependencies"
```

### Auto-Learning

When `auto_learn = true` in config, CodeCleaner analyzes AI review comments for recurring patterns:

1. After each review, comments are grouped by category
2. Common keywords are extracted across similar comments
3. Patterns appearing 3+ times become candidate rules
4. New rules start with `confidence = 0.5`
5. Confidence increases by `0.1` each time the rule matches again
6. Rules below `min_confidence` are auto-disabled after 30 days of no matches

Learned rules are appended to `rules.toml` with `source = "learned"` so you can inspect and edit them.

---

## Project Structure

```
codecleaner/
├── Cargo.toml              # Dependencies and build config
├── config.example.toml     # Example configuration (copy to config.toml)
├── rules.toml              # Review rules (manual + auto-learned)
├── src/
│   ├── main.rs             # Entry point, command dispatch
│   ├── cli.rs              # CLI argument parsing (clap)
│   ├── config.rs           # Configuration loading and validation
│   ├── error.rs            # Error types
│   ├── orchestrator/
│   │   ├── mod.rs          # PR resolution (by ID or branch)
│   │   ├── review.rs       # Review mode workflow
│   │   └── fix.rs          # Fix mode workflow
│   ├── azure/
│   │   ├── client.rs       # Azure DevOps REST API client
│   │   ├── types.rs        # API request/response types
│   │   ├── diff.rs         # Unified diff parser
│   │   └── comments.rs     # Comment extraction and filtering
│   ├── gemini/
│   │   ├── client.rs       # Gemini API client
│   │   ├── types.rs        # API types + AI output structures
│   │   ├── prompt.rs       # Prompt templates (review + fix)
│   │   └── chunker.rs      # Diff chunking for context window
│   ├── rules/
│   │   ├── engine.rs       # Rule matching engine
│   │   ├── types.rs        # Rule data structures
│   │   ├── store.rs        # Rules file I/O
│   │   └── learning.rs     # Pattern extraction from AI reviews
│   ├── review/
│   │   ├── comment.rs      # Comment deduplication
│   │   ├── mapper.rs       # AI output → Azure DevOps threads
│   │   └── formatter.rs    # Markdown comment formatting
│   ├── fix/
│   │   ├── analyzer.rs     # Review comment validation via AI
│   │   ├── patcher.rs      # Code fix application + backups
│   │   ├── git.rs          # Git operations (stage, commit, restore)
│   │   └── report.rs       # Interactive terminal report
│   ├── transport/
│   │   ├── rate_limiter.rs # Token-bucket rate limiter
│   │   ├── retry.rs        # Exponential backoff retry
│   │   └── cache.rs        # LRU response cache
│   └── ui/
│       ├── progress.rs     # Progress bars and spinners
│       ├── prompt.rs       # Interactive user prompts
│       └── report.rs       # Summary reports
```

---

## Performance & Memory Efficiency

CodeCleaner is designed to run in the background without impacting your system:

- **Streaming diffs** — Diff responses are streamed and parsed incrementally, never buffered entirely in memory
- **Bounded concurrency** — Semaphore-controlled concurrent operations prevent resource spikes
- **Diff size gating** — Files larger than `max_diff_size_bytes` (default 512 KB) are automatically skipped
- **Eager memory release** — Diff data is dropped immediately after AI returns its analysis
- **Connection pooling** — Single `reqwest::Client` instance reuses HTTP connections
- **LRU caching** — API responses are cached with configurable TTL to avoid redundant calls
- **Optimized binary** — Release builds use `lto = "thin"`, `codegen-units = 1`, and `strip = true` for a small, fast executable

---

## Comparison with CodeRabbit

| | CodeRabbit | CodeCleaner |
|---|---|---|
| **Hosting** | Cloud SaaS | Fully local |
| **Platform** | GitHub, GitLab, Bitbucket, Azure DevOps | Azure DevOps |
| **AI Model** | Vendor-chosen | Your Gemini API key |
| **Fix Mode** | Review only | Review + auto-fix |
| **Learning** | Cloud-based | Local rules.toml you own |
| **Trigger** | Automatic webhook | On-demand CLI |
| **Cost** | $12-24/user/month | Free (Gemini API costs only) |
| **Privacy** | Code passes through vendor infra | Only diffs sent to Gemini |

---

## License

MIT
