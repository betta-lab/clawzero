use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "clawzero", version, about = "Ultra-fast AI agent CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Model to use (format: provider/model)
    #[arg(short, long, global = true)]
    pub model: Option<String>,

    /// Resume an existing session by ID
    #[arg(long, global = true)]
    pub resume: Option<String>,

    /// Disable TUI and use plain text mode
    #[arg(long, global = true)]
    pub no_tui: bool,

    /// One-shot prompt (if no subcommand given)
    #[arg(trailing_var_arg = true)]
    pub prompt: Vec<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start an interactive chat session
    Chat,
    /// Show current configuration
    Config,
    /// Manage sessions
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Start gateway (Slack/Discord bot)
    Gateway {
        /// Platform to run: slack, discord (omit for all configured)
        platform: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum SessionAction {
    /// List all sessions
    List,
    /// Resume a previous session
    Resume {
        /// Session ID to resume
        id: String,
    },
}
