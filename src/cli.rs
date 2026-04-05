use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "codecleaner",
    about = "AI-powered code review tool for Azure DevOps",
    version
)]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config.toml")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Review code changes in a PR and post AI comments to Azure DevOps
    Review {
        #[command(flatten)]
        pr: PrSelector,

        /// Print review comments without posting to Azure DevOps
        #[arg(long)]
        dry_run: bool,
    },

    /// Fix existing review comments on a PR locally
    Fix {
        #[command(flatten)]
        pr: PrSelector,
    },

    /// Manage review rules
    Rules {
        #[command(subcommand)]
        action: RulesAction,
    },

    /// Validate configuration file
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(clap::Args, Clone)]
#[group(required = true, multiple = false)]
pub struct PrSelector {
    /// Pull request ID
    #[arg(long)]
    pub pr: Option<u64>,

    /// Source branch name
    #[arg(long)]
    pub branch: Option<String>,
}

#[derive(Subcommand)]
pub enum RulesAction {
    /// List all review rules
    List,
    /// Add a new review rule interactively
    Add,
    /// Remove a rule by ID
    Remove {
        /// Rule ID to remove
        id: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Validate the configuration file
    Validate,
}
