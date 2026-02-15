use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "clawzero", version, about = "Ultra-fast AI agent CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Model to use (format: provider/model)
    #[arg(short, long, global = true)]
    pub model: Option<String>,

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
}
