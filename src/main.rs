use std::io::IsTerminal;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use clawzero::agent::factory::AgentFactory;
use clawzero::cli::args::{Cli, Command, SessionAction};
use clawzero::cli::repl;
use clawzero::cli::tui;
use clawzero::config::loader::load_config;
use clawzero::gateway::session_map::SessionMap;
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
                    println!("{:<30} {:<40} {:<8}", "SESSION ID", "MODEL", "MESSAGES");
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
                let model_spec = cli.model.unwrap_or_else(|| config.defaults.model.clone());
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

    let model_spec = cli.model.unwrap_or_else(|| config.defaults.model.clone());

    let registry = ProviderRegistry::from_config(&config)?;
    let (provider, model) = registry.resolve(&model_spec)?;

    // Determine whether to use TUI: enabled by default, disabled with --no-tui or non-TTY stdin
    let use_tui = !cli.no_tui && std::io::stdin().is_terminal();

    match cli.command {
        Some(Command::Chat) => {
            if use_tui {
                let factory = AgentFactory::new(
                    provider,
                    model,
                    config.defaults.max_tokens,
                    config.defaults.max_turns,
                    config.defaults.context_limit,
                    config.tools.clone(),
                );
                let store = SessionStore::new().ok();
                tui::run_tui_repl(&factory, store.as_ref()).await?;
            } else {
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
        }
        Some(Command::Config) => {
            println!("Default model: {}", config.defaults.model);
            println!("Max tokens: {}", config.defaults.max_tokens);
            println!("Max turns: {}", config.defaults.max_turns);
            println!("Context limit: {}", config.defaults.context_limit);
            println!("Plugin tools: {}", config.tools.len());
            println!("Providers:");
            for (name, pconfig) in &config.providers {
                println!("  {name}: {:?} @ {}", pconfig.protocol, pconfig.base_url);
            }
        }
        Some(Command::Gateway { platform }) => {
            let has_slack = config.gateway.slack.is_some();
            let has_discord = config.gateway.discord.is_some();
            let has_webui = config.gateway.webui.is_some();

            let factory = Arc::new(AgentFactory::new(
                provider,
                model,
                config.defaults.max_tokens,
                config.defaults.max_turns,
                config.defaults.context_limit,
                config.tools.clone(),
            ));
            let session_map = Arc::new(SessionMap::new()?);

            match platform.as_deref() {
                Some("slack") if !has_slack => {
                    eprintln!("Slack gateway not configured. Add [gateway.slack] to config.");
                }
                Some("webui") if !has_webui => {
                    eprintln!("WebUI gateway not configured. Add [gateway.webui] to config.");
                }
                Some("discord") if !has_discord => {
                    eprintln!("Discord gateway not configured. Add [gateway.discord] to config.");
                }
                Some("slack") => {
                    let session_store = SessionStore::new()?;
                    clawzero::gateway::slack::handler::run_slack_gateway(
                        factory,
                        session_store,
                        session_map,
                        config.gateway.slack.as_ref().unwrap(),
                    )
                    .await?;
                }
                Some("discord") => {
                    let session_store = SessionStore::new()?;
                    clawzero::gateway::discord::handler::run_discord_gateway(
                        factory,
                        session_store,
                        session_map,
                        config.gateway.discord.as_ref().unwrap(),
                    )
                    .await?;
                }
                Some("webui") => {
                    let session_store = SessionStore::new()?;
                    clawzero::gateway::webui::handler::run_webui_gateway(
                        factory,
                        session_store,
                        session_map,
                        config.gateway.webui.as_ref().unwrap(),
                    )
                    .await?;
                }
                None if !has_slack && !has_discord && !has_webui => {
                    eprintln!(
                        "No gateways configured. Add [gateway.slack], [gateway.discord], or [gateway.webui] to config."
                    );
                }
                None => {
                    run_all_gateways(factory, session_map, &config).await?;
                }
                Some(other) => {
                    eprintln!(
                        "Unknown gateway platform: {other}. Use 'slack', 'discord', or 'webui'."
                    );
                }
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
                if use_tui {
                    let factory = AgentFactory::new(
                        provider,
                        model,
                        config.defaults.max_tokens,
                        config.defaults.max_turns,
                        config.defaults.context_limit,
                        config.tools.clone(),
                    );
                    let store = SessionStore::new().ok();
                    tui::run_tui_repl(&factory, store.as_ref()).await?;
                } else {
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
            } else if use_tui {
                let factory = AgentFactory::new(
                    provider,
                    model,
                    config.defaults.max_tokens,
                    config.defaults.max_turns,
                    config.defaults.context_limit,
                    config.tools.clone(),
                );
                let store = SessionStore::new().ok();
                tui::run_tui_oneshot(&factory, store.as_ref(), prompt).await?;
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

/// Run all configured gateways concurrently using tokio::select!
async fn run_all_gateways(
    factory: Arc<AgentFactory>,
    session_map: Arc<SessionMap>,
    config: &clawzero::config::types::AppConfig,
) -> Result<()> {
    let slack_fut = async {
        if let Some(ref slack_config) = config.gateway.slack {
            let session_store = SessionStore::new()?;
            return clawzero::gateway::slack::handler::run_slack_gateway(
                Arc::clone(&factory),
                session_store,
                Arc::clone(&session_map),
                slack_config,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e));
        }
        std::future::pending::<Result<()>>().await
    };

    let discord_fut = async {
        if let Some(ref discord_config) = config.gateway.discord {
            let session_store = SessionStore::new()?;
            return clawzero::gateway::discord::handler::run_discord_gateway(
                Arc::clone(&factory),
                session_store,
                Arc::clone(&session_map),
                discord_config,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e));
        }
        std::future::pending::<Result<()>>().await
    };

    let webui_fut = async {
        if let Some(ref webui_config) = config.gateway.webui {
            let session_store = SessionStore::new()?;
            return clawzero::gateway::webui::handler::run_webui_gateway(
                Arc::clone(&factory),
                session_store,
                Arc::clone(&session_map),
                webui_config,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e));
        }
        std::future::pending::<Result<()>>().await
    };

    println!("Starting gateways...");
    if config.gateway.slack.is_some() {
        println!("  - Slack (Socket Mode)");
    }
    if config.gateway.discord.is_some() {
        println!("  - Discord");
    }
    if config.gateway.webui.is_some() {
        println!("  - WebUI (HTTP + WebSocket)");
    }

    tokio::select! {
        result = slack_fut => {
            eprintln!("Slack gateway exited");
            result
        }
        result = discord_fut => {
            eprintln!("Discord gateway exited");
            result
        }
        result = webui_fut => {
            eprintln!("WebUI gateway exited");
            result
        }
    }
}
