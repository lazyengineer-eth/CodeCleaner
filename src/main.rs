mod azure;
mod cli;
mod config;
mod error;
mod fix;
mod gemini;
mod orchestrator;
mod review;
mod rules;
mod transport;
mod ui;

use clap::Parser;
use cli::{Cli, Commands, ConfigAction, RulesAction};
use colored::Colorize;
use config::AppConfig;
use tracing::error;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Handle config validate before loading full config
    if let Commands::Config {
        action: ConfigAction::Validate,
    } = &cli.command
    {
        match AppConfig::load(&cli.config) {
            Ok(_) => {
                println!("{} Configuration is valid", "✓".green().bold());
                return;
            }
            Err(e) => {
                eprintln!("{} {e}", "✗".red().bold());
                std::process::exit(1);
            }
        }
    }

    // Load config
    let config = match AppConfig::load(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} Failed to load config: {e}", "Error:".red().bold());
            std::process::exit(1);
        }
    };

    // Setup logging
    setup_logging(&config);

    // Handle rules commands (don't need API clients)
    if let Commands::Rules { action } = &cli.command {
        handle_rules_command(action, &config);
        return;
    }

    // Build API clients
    let azure = match azure::client::AzureClient::new(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} Failed to create Azure client: {e}", "Error:".red().bold());
            std::process::exit(1);
        }
    };

    let gemini_client = match gemini::client::GeminiClient::new(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} Failed to create Gemini client: {e}", "Error:".red().bold());
            std::process::exit(1);
        }
    };

    // Load rules
    let mut rules_file = match rules::store::load_rules(&config.rules.file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Failed to load rules: {e}", "Warning:".yellow().bold());
            rules::store::default_rules()
        }
    };

    // Execute command
    let result = match &cli.command {
        Commands::Review { pr, dry_run } => {
            match orchestrator::resolve_pr(&azure, pr).await {
                Ok(pull_request) => {
                    println!(
                        "\n{} PR #{}: {}",
                        "Reviewing".cyan().bold(),
                        pull_request.pull_request_id,
                        pull_request.title
                    );
                    orchestrator::review::run_review(
                        &config,
                        &azure,
                        &gemini_client,
                        &pull_request,
                        &mut rules_file,
                        *dry_run,
                    )
                    .await
                }
                Err(e) => Err(e),
            }
        }
        Commands::Fix { pr } => {
            match orchestrator::resolve_pr(&azure, pr).await {
                Ok(pull_request) => {
                    println!(
                        "\n{} PR #{}: {}",
                        "Fixing".cyan().bold(),
                        pull_request.pull_request_id,
                        pull_request.title
                    );
                    orchestrator::fix::run_fix(&config, &azure, &gemini_client, &pull_request)
                        .await
                }
                Err(e) => Err(e),
            }
        }
        Commands::Rules { .. } | Commands::Config { .. } => unreachable!(),
    };

    if let Err(e) = result {
        error!(error = %e, "Command failed");
        eprintln!("\n{} {e}", "Error:".red().bold());
        std::process::exit(1);
    }
}

fn setup_logging(config: &AppConfig) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact();

    if config.logging.log_to_file {
        let file_appender =
            tracing_appender::rolling::daily(".", &config.logging.file);
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        // We leak the guard intentionally to keep the logger alive for the process lifetime
        let guard = Box::new(_guard);
        std::mem::forget(guard);

        subscriber.with_writer(non_blocking).init();
    } else {
        subscriber.with_writer(std::io::stderr).init();
    }
}

fn handle_rules_command(action: &RulesAction, config: &AppConfig) {
    match action {
        RulesAction::List => {
            let rules_file = match rules::store::load_rules(&config.rules.file) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{} {e}", "Error:".red().bold());
                    return;
                }
            };

            println!("\n{}", "Review Rules".bold().underline());
            println!(
                "File: {} | Total reviews: {}\n",
                config.rules.file.display(),
                rules_file.meta.total_reviews
            );

            if rules_file.skip.is_empty() {
                println!("  No skip rules defined.\n");
            } else {
                println!("{}", "Skip Rules:".bold());
                for skip in &rules_file.skip {
                    println!("  {} — {}", skip.glob.cyan(), skip.reason.dimmed());
                }
                println!();
            }

            if rules_file.rule.is_empty() {
                println!("  No review rules defined.");
            } else {
                println!("{}", "Review Rules:".bold());
                for rule in &rules_file.rule {
                    let status = if rule.enabled {
                        "ON".green()
                    } else {
                        "OFF".red()
                    };
                    let source = if rule.source == "learned" {
                        format!("learned (conf: {:.1})", rule.confidence).yellow()
                    } else {
                        "manual".normal()
                    };
                    println!(
                        "  [{}] {} {} ({}) — {} [{}]",
                        status,
                        rule.id.bright_cyan(),
                        rule.name,
                        rule.severity,
                        rule.message_template.dimmed(),
                        source
                    );
                }
            }
            println!();
        }
        RulesAction::Add => {
            println!("Interactive rule addition not yet implemented.");
            println!("Edit '{}' directly to add rules.", config.rules.file.display());
        }
        RulesAction::Remove { id } => {
            match rules::store::remove_rule(&config.rules.file, id) {
                Ok(true) => println!("{} Removed rule '{id}'", "✓".green().bold()),
                Ok(false) => println!("Rule '{id}' not found"),
                Err(e) => eprintln!("{} {e}", "Error:".red().bold()),
            }
        }
    }
}
