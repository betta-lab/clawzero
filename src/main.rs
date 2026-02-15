use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use clawzero::cli::args::{Cli, Command, SessionAction};
use clawzero::cli::repl;
use clawzero::config::loader::load_config;
use clawzero::provider::registry::ProviderRegistry;
use clawzero::session::store::SessionStore;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let config = load_config()?;

    // Handle session subcommands (don't need provider)
    if let Some(Command::Sessions { action }) = &cli.command {
        let store = SessionStore::new()?;
        match action {
            SessionAction::List => {
                let sessions = store.list_sessions()?;
                if sessions.is_empty() {
                    println!("No sessions found.");
                } else {
                    println!(
                        "{:<30} {:<40} {:<8}",
                        "SESSION ID", "MODEL", "MESSAGES"
                    );
                    for s in &sessions {
                        println!(
                            "{:<30} {:<40} {:<8}",
                            s.session_id, s.model, s.message_count
                        );
                    }
                }
                return Ok(());
            }
            SessionAction::Resume { id } => {
                let model_spec = cli
                    .model
                    .unwrap_or_else(|| config.defaults.model.clone());
                let registry = ProviderRegistry::from_config(&config)?;
                let (provider, model) = registry.resolve(&model_spec)?;

                repl::run_repl_resume(
                    provider,
                    model,
                    config.defaults.max_tokens,
                    config.defaults.max_turns,
                    config.defaults.context_limit,
                    &config.tools,
                    &store,
                    id,
                )
                .await;
                return Ok(());
            }
        }
    }

    let model_spec = cli
        .model
        .unwrap_or_else(|| config.defaults.model.clone());

    let registry = ProviderRegistry::from_config(&config)?;
    let (provider, model) = registry.resolve(&model_spec)?;

    match cli.command {
        Some(Command::Chat) => {
            repl::run_repl(
                provider,
                model,
                config.defaults.max_tokens,
                config.defaults.max_turns,
                config.defaults.context_limit,
                &config.tools,
            )
            .await;
        }
        Some(Command::Config) => {
            println!("Default model: {}", config.defaults.model);
            println!("Max tokens: {}", config.defaults.max_tokens);
            println!("Max turns: {}", config.defaults.max_turns);
            println!("Context limit: {}", config.defaults.context_limit);
            println!("Plugin tools: {}", config.tools.len());
            println!("Providers:");
            for (name, pconfig) in &config.providers {
                println!(
                    "  {name}: {:?} @ {}",
                    pconfig.protocol, pconfig.base_url
                );
            }
        }
        Some(Command::Sessions { .. }) => unreachable!(), // Handled above
        None => {
            let prompt = cli.prompt.join(" ");
            if let Some(resume_id) = cli.resume {
                let store = SessionStore::new()?;
                repl::run_repl_resume(
                    provider,
                    model,
                    config.defaults.max_tokens,
                    config.defaults.max_turns,
                    config.defaults.context_limit,
                    &config.tools,
                    &store,
                    &resume_id,
                )
                .await;
            } else if prompt.is_empty() {
                repl::run_repl(
                    provider,
                    model,
                    config.defaults.max_tokens,
                    config.defaults.max_turns,
                    config.defaults.context_limit,
                    &config.tools,
                )
                .await;
            } else {
                repl::run_oneshot(
                    provider,
                    model,
                    prompt,
                    config.defaults.max_tokens,
                    config.defaults.max_turns,
                    config.defaults.context_limit,
                    &config.tools,
                )
                .await;
            }
        }
    }

    Ok(())
}
