use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use clawzero::cli::args::{Cli, Command};
use clawzero::cli::repl;
use clawzero::config::loader::load_config;
use clawzero::provider::registry::ProviderRegistry;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let config = load_config()?;

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
            )
            .await;
        }
        Some(Command::Config) => {
            println!("Default model: {}", config.defaults.model);
            println!("Max tokens: {}", config.defaults.max_tokens);
            println!("Max turns: {}", config.defaults.max_turns);
            println!("Providers:");
            for (name, pconfig) in &config.providers {
                println!(
                    "  {name}: {:?} @ {}",
                    pconfig.protocol, pconfig.base_url
                );
            }
        }
        None => {
            let prompt = cli.prompt.join(" ");
            if prompt.is_empty() {
                // No prompt and no subcommand -> start REPL
                repl::run_repl(
                    provider,
                    model,
                    config.defaults.max_tokens,
                    config.defaults.max_turns,
                )
                .await;
            } else {
                repl::run_oneshot(
                    provider,
                    model,
                    prompt,
                    config.defaults.max_tokens,
                    config.defaults.max_turns,
                )
                .await;
            }
        }
    }

    Ok(())
}
