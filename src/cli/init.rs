use std::io;
use std::path::Path;

use console::style;

use crate::error::ClawError;

// ---------------------------------------------------------------------------
// Provider metadata (table-driven)
// ---------------------------------------------------------------------------

struct ProviderMeta {
    name: &'static str,
    display: &'static str,
    protocol: &'static str,
    base_url: &'static str,
    needs_api_key: bool,
    api_key_env: Option<&'static str>,
    auth: Option<&'static str>,
    needs_project_id: bool,
    needs_region: bool,
    default_region: Option<&'static str>,
    models: &'static [&'static str],
}

const PROVIDERS: &[ProviderMeta] = &[
    ProviderMeta {
        name: "anthropic",
        display: "Anthropic",
        protocol: "anthropic",
        base_url: "https://api.anthropic.com",
        needs_api_key: true,
        api_key_env: Some("ANTHROPIC_API_KEY"),
        auth: None,
        needs_project_id: false,
        needs_region: false,
        default_region: None,
        models: &[
            "claude-opus-4-6",
            "claude-sonnet-4-5-20250929",
            "claude-haiku-4-5-20251001",
            "claude-sonnet-4-20250514",
        ],
    },
    ProviderMeta {
        name: "openai",
        display: "OpenAI",
        protocol: "openai",
        base_url: "https://api.openai.com",
        needs_api_key: true,
        api_key_env: Some("OPENAI_API_KEY"),
        auth: None,
        needs_project_id: false,
        needs_region: false,
        default_region: None,
        models: &[
            "gpt-5.2",
            "gpt-4.1",
            "gpt-4.1-mini",
            "o3",
            "o4-mini",
            "gpt-4o",
            "gpt-4o-mini",
        ],
    },
    ProviderMeta {
        name: "openrouter",
        display: "OpenRouter",
        protocol: "openai",
        base_url: "https://openrouter.ai/api",
        needs_api_key: true,
        api_key_env: Some("OPENROUTER_API_KEY"),
        auth: None,
        needs_project_id: false,
        needs_region: false,
        default_region: None,
        models: &[
            "anthropic/claude-opus-4.6",
            "anthropic/claude-sonnet-4.5",
            "anthropic/claude-haiku-4.5",
            "google/gemini-2.5-pro",
            "google/gemini-2.5-flash",
            "deepseek/deepseek-r1",
            "meta-llama/llama-3.3-70b-instruct",
        ],
    },
    ProviderMeta {
        name: "ollama",
        display: "Ollama (local)",
        protocol: "openai",
        base_url: "http://localhost:11434",
        needs_api_key: false,
        api_key_env: None,
        auth: None,
        needs_project_id: false,
        needs_region: false,
        default_region: None,
        models: &[
            "llama3.3",
            "qwen3",
            "gemma3",
            "deepseek-r1",
            "phi4",
            "qwen2.5-coder",
            "codellama",
            "mistral",
        ],
    },
    ProviderMeta {
        name: "vertex",
        display: "Vertex AI (Google Cloud)",
        protocol: "anthropic",
        base_url: "https://{region}-aiplatform.googleapis.com",
        needs_api_key: false,
        api_key_env: None,
        auth: Some("vertex"),
        needs_project_id: true,
        needs_region: true,
        default_region: Some("us-central1"),
        models: &[
            "claude-opus-4-6",
            "claude-sonnet-4-5@20250929",
            "claude-haiku-4-5@20251001",
            "claude-sonnet-4@20250514",
        ],
    },
    ProviderMeta {
        name: "bedrock",
        display: "Bedrock (AWS)",
        protocol: "anthropic",
        base_url: "https://bedrock-runtime.{region}.amazonaws.com",
        needs_api_key: false,
        api_key_env: None,
        auth: Some("bedrock"),
        needs_project_id: false,
        needs_region: true,
        default_region: Some("us-east-1"),
        models: &[
            "anthropic.claude-opus-4-6-v1",
            "anthropic.claude-sonnet-4-5-20250929-v1:0",
            "anthropic.claude-haiku-4-5-20251001-v1:0",
            "anthropic.claude-sonnet-4-20250514-v1:0",
        ],
    },
];

// ---------------------------------------------------------------------------
// Gateway metadata
// ---------------------------------------------------------------------------

struct GatewayMeta {
    name: &'static str,
    display: &'static str,
}

const GATEWAYS: &[GatewayMeta] = &[
    GatewayMeta {
        name: "slack",
        display: "Slack",
    },
    GatewayMeta {
        name: "discord",
        display: "Discord",
    },
    GatewayMeta {
        name: "webui",
        display: "WebUI",
    },
];

// ---------------------------------------------------------------------------
// Answer types
// ---------------------------------------------------------------------------

pub struct ProviderAnswer {
    pub name: String,
    pub protocol: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub api_key_env: Option<String>,
    pub auth: Option<String>,
    pub project_id: Option<String>,
    pub region: Option<String>,
}

pub struct GatewayAnswer {
    pub name: String,
    pub string_fields: Vec<(String, String)>,
    pub int_fields: Vec<(String, u16)>,
}

pub struct InitAnswers {
    pub providers: Vec<ProviderAnswer>,
    pub default_model: String,
    pub gateways: Vec<GatewayAnswer>,
}

// ---------------------------------------------------------------------------
// Prompter trait
// ---------------------------------------------------------------------------

pub trait InitPrompter {
    fn println(&mut self, msg: &str);
    fn read_password(&mut self, prompt: &str) -> io::Result<String>;
    fn confirm(&mut self, prompt: &str, default: bool) -> io::Result<bool>;
    fn read_input(&mut self, prompt: &str, default: Option<&str>) -> io::Result<String>;
    fn multi_select(
        &mut self,
        prompt: &str,
        items: &[&str],
        defaults: &[bool],
    ) -> io::Result<Vec<usize>>;
    fn select(&mut self, prompt: &str, items: &[String], default: usize) -> io::Result<usize>;
}

// ---------------------------------------------------------------------------
// StdioPrompter (dialoguer)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct StdioPrompter {
    theme: dialoguer::theme::ColorfulTheme,
}

impl StdioPrompter {
    pub fn new() -> Self {
        Self::default()
    }
}

fn dialoguer_err(e: dialoguer::Error) -> io::Error {
    match e {
        dialoguer::Error::IO(io_err) => io_err,
    }
}

impl InitPrompter for StdioPrompter {
    fn println(&mut self, msg: &str) {
        println!("{msg}");
    }

    fn read_password(&mut self, prompt: &str) -> io::Result<String> {
        dialoguer::Password::with_theme(&self.theme)
            .with_prompt(prompt)
            .allow_empty_password(true)
            .interact()
            .map_err(dialoguer_err)
    }

    fn confirm(&mut self, prompt: &str, default: bool) -> io::Result<bool> {
        dialoguer::Confirm::with_theme(&self.theme)
            .with_prompt(prompt)
            .default(default)
            .interact()
            .map_err(dialoguer_err)
    }

    fn read_input(&mut self, prompt: &str, default: Option<&str>) -> io::Result<String> {
        let mut input = dialoguer::Input::<String>::with_theme(&self.theme).with_prompt(prompt);
        if let Some(def) = default {
            input = input.default(def.to_string());
        }
        input.interact_text().map_err(dialoguer_err)
    }

    fn multi_select(
        &mut self,
        prompt: &str,
        items: &[&str],
        defaults: &[bool],
    ) -> io::Result<Vec<usize>> {
        dialoguer::MultiSelect::with_theme(&self.theme)
            .with_prompt(prompt)
            .items(items)
            .defaults(defaults)
            .interact()
            .map_err(dialoguer_err)
    }

    fn select(&mut self, prompt: &str, items: &[String], default: usize) -> io::Result<usize> {
        dialoguer::Select::with_theme(&self.theme)
            .with_prompt(prompt)
            .items(items)
            .default(default)
            .interact()
            .map_err(dialoguer_err)
    }
}

// ---------------------------------------------------------------------------
// Config generation (pure function)
// ---------------------------------------------------------------------------

pub fn generate_config_toml(answers: &InitAnswers) -> String {
    let mut out = String::from("# clawzero configuration\n# Generated by `clawzero init`\n\n");

    out.push_str("[defaults]\n");
    out.push_str(&format!("model = \"{}\"\n", answers.default_model));
    out.push_str("max_tokens = 8192\n");

    for provider in &answers.providers {
        out.push_str(&format!("\n[providers.{}]\n", provider.name));
        out.push_str(&format!("protocol = \"{}\"\n", provider.protocol));
        out.push_str(&format!("base_url = \"{}\"\n", provider.base_url));
        if let Some(ref key) = provider.api_key {
            out.push_str(&format!("api_key = \"{key}\"\n"));
        }
        if let Some(ref env) = provider.api_key_env {
            out.push_str(&format!("api_key_env = \"{env}\"\n"));
        }
        if let Some(ref auth) = provider.auth {
            out.push_str(&format!("auth = \"{auth}\"\n"));
        }
        if let Some(ref pid) = provider.project_id {
            out.push_str(&format!("project_id = \"{pid}\"\n"));
        }
        if let Some(ref region) = provider.region {
            out.push_str(&format!("region = \"{region}\"\n"));
        }
    }

    for gw in &answers.gateways {
        out.push_str(&format!("\n[gateway.{}]\n", gw.name));
        for (key, val) in &gw.string_fields {
            out.push_str(&format!("{key} = \"{val}\"\n"));
        }
        for (key, val) in &gw.int_fields {
            out.push_str(&format!("{key} = {val}\n"));
        }
    }

    out
}

// ---------------------------------------------------------------------------
// File writer
// ---------------------------------------------------------------------------

pub fn write_config(path: &Path, content: &str) -> Result<(), ClawError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Per-provider input collection
// ---------------------------------------------------------------------------

fn collect_provider_answer(
    prompter: &mut dyn InitPrompter,
    meta: &ProviderMeta,
) -> Result<ProviderAnswer, ClawError> {
    let map_err = |e: io::Error| ClawError::Config(format!("Failed to read input: {e}"));

    let mut api_key = None;
    let mut api_key_env = None;

    if meta.needs_api_key {
        // Anthropic: offer auth method selection (API Key vs setup-token)
        if meta.name == "anthropic" {
            let auth_choices = vec!["API Key".to_string(), "Claude Code setup-token".to_string()];
            let auth_idx = prompter
                .select(
                    &format!("{} authentication method", meta.display),
                    &auth_choices,
                    0,
                )
                .map_err(map_err)?;
            match auth_idx {
                1 => {
                    // setup-token
                    let token = prompter
                        .read_password("Setup token (sk-ant-oat01-...)")
                        .map_err(map_err)?;
                    let token = token.trim().to_string();
                    if token.is_empty() {
                        if let Some(env) = meta.api_key_env {
                            api_key_env = Some(env.to_string());
                        }
                    } else {
                        api_key = Some(token);
                    }
                }
                _ => {
                    // Standard API Key (existing flow)
                    let hint = if let Some(env) = meta.api_key_env {
                        format!("{} API key (leave empty to use ${env})", meta.display)
                    } else {
                        format!("{} API key", meta.display)
                    };
                    let key = prompter.read_password(&hint).map_err(map_err)?;
                    let key = key.trim().to_string();
                    if key.is_empty() {
                        if let Some(env) = meta.api_key_env {
                            api_key_env = Some(env.to_string());
                        }
                    } else {
                        api_key = Some(key);
                    }
                }
            }
        } else {
            let hint = if let Some(env) = meta.api_key_env {
                format!("{} API key (leave empty to use ${env})", meta.display)
            } else {
                format!("{} API key", meta.display)
            };
            let key = prompter.read_password(&hint).map_err(map_err)?;
            let key = key.trim().to_string();
            if key.is_empty() {
                if let Some(env) = meta.api_key_env {
                    api_key_env = Some(env.to_string());
                }
            } else {
                api_key = Some(key);
            }
        }
    }

    let mut base_url = meta.base_url.to_string();
    let mut region = None;
    let mut project_id = None;

    if meta.needs_region {
        let default = meta.default_region.unwrap_or("us-east-1");
        let r = prompter
            .read_input(&format!("{} region", meta.display), Some(default))
            .map_err(map_err)?;
        base_url = base_url.replace("{region}", &r);
        region = Some(r);
    }

    if meta.needs_project_id {
        let pid = prompter
            .read_input(&format!("{} project ID", meta.display), None)
            .map_err(map_err)?;
        project_id = Some(pid);
    }

    if meta.name == "ollama" {
        let url = prompter
            .read_input("Ollama base URL", Some(meta.base_url))
            .map_err(map_err)?;
        base_url = url;
    }

    Ok(ProviderAnswer {
        name: meta.name.to_string(),
        protocol: meta.protocol.to_string(),
        base_url,
        api_key,
        api_key_env,
        auth: meta.auth.map(|s| s.to_string()),
        project_id,
        region,
    })
}

// ---------------------------------------------------------------------------
// Per-gateway input collection
// ---------------------------------------------------------------------------

fn collect_gateway_answer(
    prompter: &mut dyn InitPrompter,
    meta: &GatewayMeta,
) -> Result<GatewayAnswer, ClawError> {
    let map_err = |e: io::Error| ClawError::Config(format!("Failed to read input: {e}"));

    let mut string_fields = Vec::new();
    let mut int_fields = Vec::new();

    match meta.name {
        "slack" => {
            let app_token = prompter
                .read_password("Slack App Token (leave empty to use $SLACK_APP_TOKEN)")
                .map_err(map_err)?;
            let app_token = app_token.trim().to_string();
            if app_token.is_empty() {
                string_fields.push(("app_token_env".into(), "SLACK_APP_TOKEN".into()));
            } else {
                string_fields.push(("app_token".into(), app_token));
            }

            let bot_token = prompter
                .read_password("Slack Bot Token (leave empty to use $SLACK_BOT_TOKEN)")
                .map_err(map_err)?;
            let bot_token = bot_token.trim().to_string();
            if bot_token.is_empty() {
                string_fields.push(("bot_token_env".into(), "SLACK_BOT_TOKEN".into()));
            } else {
                string_fields.push(("bot_token".into(), bot_token));
            }
        }
        "discord" => {
            let bot_token = prompter
                .read_password("Discord Bot Token (leave empty to use $DISCORD_BOT_TOKEN)")
                .map_err(map_err)?;
            let bot_token = bot_token.trim().to_string();
            if bot_token.is_empty() {
                string_fields.push(("bot_token_env".into(), "DISCORD_BOT_TOKEN".into()));
            } else {
                string_fields.push(("bot_token".into(), bot_token));
            }
        }
        "webui" => {
            let host = prompter
                .read_input("WebUI host", Some("127.0.0.1"))
                .map_err(map_err)?;
            string_fields.push(("host".into(), host));

            let port_str = prompter
                .read_input("WebUI port", Some("3000"))
                .map_err(map_err)?;
            let port: u16 = port_str.parse().unwrap_or(3000);
            int_fields.push(("port".into(), port));
        }
        _ => {}
    }

    Ok(GatewayAnswer {
        name: meta.name.to_string(),
        string_fields,
        int_fields,
    })
}

// ---------------------------------------------------------------------------
// Main init flow
// ---------------------------------------------------------------------------

const BOX_INNER_WIDTH: usize = 43;

fn box_top() -> String {
    format!(
        "  {}",
        style(format!(
            "\u{256d}{}\u{256e}",
            "\u{2500}".repeat(BOX_INNER_WIDTH + 2)
        ))
        .dim()
    )
}

fn box_bottom() -> String {
    format!(
        "  {}",
        style(format!(
            "\u{2570}{}\u{256f}",
            "\u{2500}".repeat(BOX_INNER_WIDTH + 2)
        ))
        .dim()
    )
}

fn box_line(content: &str) -> String {
    let visible = console::measure_text_width(content);
    let pad = BOX_INNER_WIDTH.saturating_sub(visible);
    format!(
        "  {} {} {}{} {}",
        style("\u{2502}").dim(),
        content,
        " ".repeat(pad),
        "",
        style("\u{2502}").dim()
    )
}

fn box_empty() -> String {
    box_line("")
}

fn provider_status_detail(answer: &ProviderAnswer, meta: &ProviderMeta) -> String {
    if answer.api_key.is_some() {
        "API key set".to_string()
    } else if let Some(ref env) = answer.api_key_env {
        format!("${env}")
    } else if meta.name == "ollama" {
        answer.base_url.clone()
    } else if let Some(ref region) = answer.region {
        region.clone()
    } else {
        "configured".to_string()
    }
}

fn gateway_status_detail(answer: &GatewayAnswer) -> String {
    let token_count = answer
        .string_fields
        .iter()
        .filter(|(k, _)| k.ends_with("token"))
        .count();
    let env_count = answer
        .string_fields
        .iter()
        .filter(|(k, _)| k.ends_with("_env"))
        .count();
    if token_count > 0 && env_count > 0 {
        format!("{token_count} token(s) + env fallback")
    } else if token_count > 0 {
        format!("{token_count} token(s) set")
    } else if env_count > 0 {
        "using env vars".to_string()
    } else {
        "configured".to_string()
    }
}

pub fn run_init(prompter: &mut dyn InitPrompter, config_path: &Path) -> Result<(), ClawError> {
    let map_err = |e: io::Error| ClawError::Config(format!("Failed to read input: {e}"));

    // ── Banner ──────────────────────────────────────────────
    prompter.println("");
    prompter.println(&format!(
        "  {} {}",
        style("\u{1f43e}").bold(),
        style("clawzero setup").cyan().bold()
    ));
    prompter.println(&format!("  {}", style("\u{2501}".repeat(30)).dim()));
    prompter.println(&format!(
        "  {}",
        style("Let's get your AI agent up and running!").dim()
    ));
    prompter.println("");

    // Check for existing config
    if config_path.exists() {
        let overwrite = prompter
            .confirm("Config file already exists. Overwrite?", true)
            .map_err(map_err)?;
        if !overwrite {
            prompter.println(&format!(
                "  {} Existing config unchanged.",
                style("Aborted.").yellow()
            ));
            return Ok(());
        }
    }

    // ── Step 1: Providers ───────────────────────────────────
    prompter.println(&format!(
        "  {}",
        style("\u{2500}\u{2500} \u{1f4e6} Step 1/3 \u{00b7} Providers \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}")
            .yellow()
            .bold()
    ));
    prompter.println("");

    let provider_displays: Vec<&str> = PROVIDERS.iter().map(|p| p.display).collect();
    let defaults: Vec<bool> = PROVIDERS.iter().map(|p| p.name == "anthropic").collect();
    let selected_indices = prompter
        .multi_select(
            "Which providers do you want to configure?",
            &provider_displays,
            &defaults,
        )
        .map_err(map_err)?;

    if selected_indices.is_empty() {
        prompter.println("");
        prompter.println(&format!(
            "  {} No providers selected. Run {} anytime to try again.",
            style("\u{1f44b}").bold(),
            style("clawzero init").cyan()
        ));
        return Ok(());
    }

    // Collect per-provider answers
    let mut providers = Vec::new();
    for &idx in &selected_indices {
        let meta = &PROVIDERS[idx];
        prompter.println("");
        let answer = collect_provider_answer(prompter, meta)?;
        let detail = provider_status_detail(&answer, meta);
        prompter.println(&format!(
            "  {} {} \u{2500} {}",
            style("\u{2713}").green().bold(),
            style(meta.display).bold(),
            style(detail).dim()
        ));
        providers.push(answer);
    }

    // ── Step 2: Default Model ───────────────────────────────
    prompter.println("");
    prompter.println(&format!(
        "  {}",
        style("\u{2500}\u{2500} \u{1f3af} Step 2/3 \u{00b7} Default Model \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}")
            .yellow()
            .bold()
    ));
    prompter.println("");

    let mut model_choices: Vec<String> = Vec::new();
    for &idx in &selected_indices {
        let meta = &PROVIDERS[idx];
        for model in meta.models {
            model_choices.push(format!("{}/{model}", meta.name));
        }
    }
    let default_model_idx = prompter
        .select("Choose your default model", &model_choices, 0)
        .map_err(map_err)?;
    let default_model = model_choices[default_model_idx].clone();

    prompter.println(&format!(
        "  {} Default: {}",
        style("\u{2713}").green().bold(),
        style(&default_model).cyan().bold()
    ));

    // ── Step 3: Gateways ────────────────────────────────────
    prompter.println("");
    prompter.println(&format!(
        "  {}",
        style("\u{2500}\u{2500} \u{1f50c} Step 3/3 \u{00b7} Gateways \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}")
            .yellow()
            .bold()
    ));
    prompter.println("");

    let mut gateways = Vec::new();
    let configure_gateways = prompter
        .confirm("Configure gateways (Slack/Discord/WebUI)?", false)
        .map_err(map_err)?;

    if configure_gateways {
        let gw_displays: Vec<&str> = GATEWAYS.iter().map(|g| g.display).collect();
        let gw_defaults = vec![false; GATEWAYS.len()];
        let gw_selected = prompter
            .multi_select("Which gateways?", &gw_displays, &gw_defaults)
            .map_err(map_err)?;

        for &idx in &gw_selected {
            let meta = &GATEWAYS[idx];
            prompter.println("");
            let answer = collect_gateway_answer(prompter, meta)?;
            let detail = gateway_status_detail(&answer);
            prompter.println(&format!(
                "  {} {} \u{2500} {}",
                style("\u{2713}").green().bold(),
                style(meta.display).bold(),
                style(detail).dim()
            ));
            gateways.push(answer);
        }
    } else {
        prompter.println(&format!(
            "  {} Skipped \u{2500} {}",
            style("\u{2500}").dim(),
            style("run `clawzero gateway` later to set up").dim()
        ));
    }

    let answers = InitAnswers {
        providers,
        default_model,
        gateways,
    };

    let toml = generate_config_toml(&answers);
    write_config(config_path, &toml)?;

    // ── Done! ───────────────────────────────────────────────
    prompter.println("");
    prompter.println(&format!(
        "  {} Config written to {}",
        style("\u{2705}"),
        style(config_path.display()).underlined()
    ));
    prompter.println("");
    prompter.println(&box_top());
    prompter.println(&box_empty());
    prompter.println(&box_line(&format!(
        "\u{1f389} {}",
        style("You're all set!").green().bold()
    )));
    prompter.println(&box_empty());
    prompter.println(&box_line("Try it out:"));
    prompter.println(&box_line(&format!(
        "  {} {}",
        style("$").dim(),
        style("clawzero \"Hello, world!\"").cyan().bold()
    )));
    prompter.println(&box_empty());
    prompter.println(&box_line("Other commands:"));
    prompter.println(&box_line(&format!(
        "  {} {} {}",
        style("$").dim(),
        style("clawzero chat").cyan(),
        style("# interactive mode").dim()
    )));
    prompter.println(&box_line(&format!(
        "  {} {} {}",
        style("$").dim(),
        style("clawzero config").cyan(),
        style("# show config").dim()
    )));
    prompter.println(&box_empty());
    prompter.println(&box_bottom());
    prompter.println("");

    Ok(())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    // -----------------------------------------------------------------------
    // MockPrompter (builder pattern)
    // -----------------------------------------------------------------------

    struct MockPrompter {
        outputs: Vec<String>,
        passwords: VecDeque<String>,
        confirms: VecDeque<bool>,
        inputs: VecDeque<String>,
        multi_selects: VecDeque<Vec<usize>>,
        selects: VecDeque<usize>,
    }

    impl MockPrompter {
        fn new() -> Self {
            Self {
                outputs: Vec::new(),
                passwords: VecDeque::new(),
                confirms: VecDeque::new(),
                inputs: VecDeque::new(),
                multi_selects: VecDeque::new(),
                selects: VecDeque::new(),
            }
        }

        fn with_passwords(mut self, passwords: Vec<&str>) -> Self {
            self.passwords = passwords.into_iter().map(|s| s.to_string()).collect();
            self
        }

        fn with_confirms(mut self, confirms: Vec<bool>) -> Self {
            self.confirms = confirms.into_iter().collect();
            self
        }

        fn with_inputs(mut self, inputs: Vec<&str>) -> Self {
            self.inputs = inputs.into_iter().map(|s| s.to_string()).collect();
            self
        }

        fn with_multi_selects(mut self, multi_selects: Vec<Vec<usize>>) -> Self {
            self.multi_selects = multi_selects.into_iter().collect();
            self
        }

        fn with_selects(mut self, selects: Vec<usize>) -> Self {
            self.selects = selects.into_iter().collect();
            self
        }
    }

    impl InitPrompter for MockPrompter {
        fn println(&mut self, msg: &str) {
            self.outputs.push(msg.to_string());
        }

        fn read_password(&mut self, _prompt: &str) -> io::Result<String> {
            self.passwords
                .pop_front()
                .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "no more passwords"))
        }

        fn confirm(&mut self, _prompt: &str, _default: bool) -> io::Result<bool> {
            self.confirms
                .pop_front()
                .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "no more confirms"))
        }

        fn read_input(&mut self, _prompt: &str, _default: Option<&str>) -> io::Result<String> {
            self.inputs
                .pop_front()
                .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "no more inputs"))
        }

        fn multi_select(
            &mut self,
            _prompt: &str,
            _items: &[&str],
            _defaults: &[bool],
        ) -> io::Result<Vec<usize>> {
            self.multi_selects.pop_front().ok_or_else(|| {
                io::Error::new(io::ErrorKind::UnexpectedEof, "no more multi_selects")
            })
        }

        fn select(
            &mut self,
            _prompt: &str,
            _items: &[String],
            _default: usize,
        ) -> io::Result<usize> {
            self.selects
                .pop_front()
                .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "no more selects"))
        }
    }

    // -----------------------------------------------------------------------
    // generate_config_toml tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_config_anthropic_only() {
        let answers = InitAnswers {
            providers: vec![ProviderAnswer {
                name: "anthropic".into(),
                protocol: "anthropic".into(),
                base_url: "https://api.anthropic.com".into(),
                api_key: Some("sk-ant-test123".into()),
                api_key_env: None,
                auth: None,
                project_id: None,
                region: None,
            }],
            default_model: "anthropic/claude-sonnet-4-20250514".into(),
            gateways: vec![],
        };
        let toml = generate_config_toml(&answers);
        assert!(toml.contains("[providers.anthropic]"));
        assert!(toml.contains("sk-ant-test123"));
        assert!(!toml.contains("[providers.openai]"));
    }

    #[test]
    fn test_generate_config_multiple_providers() {
        let answers = InitAnswers {
            providers: vec![
                ProviderAnswer {
                    name: "anthropic".into(),
                    protocol: "anthropic".into(),
                    base_url: "https://api.anthropic.com".into(),
                    api_key: Some("sk-ant-test".into()),
                    api_key_env: None,
                    auth: None,
                    project_id: None,
                    region: None,
                },
                ProviderAnswer {
                    name: "openai".into(),
                    protocol: "openai".into(),
                    base_url: "https://api.openai.com".into(),
                    api_key: Some("sk-openai-test".into()),
                    api_key_env: None,
                    auth: None,
                    project_id: None,
                    region: None,
                },
                ProviderAnswer {
                    name: "openrouter".into(),
                    protocol: "openai".into(),
                    base_url: "https://openrouter.ai/api".into(),
                    api_key: Some("sk-or-test".into()),
                    api_key_env: None,
                    auth: None,
                    project_id: None,
                    region: None,
                },
            ],
            default_model: "anthropic/claude-sonnet-4-20250514".into(),
            gateways: vec![],
        };
        let toml = generate_config_toml(&answers);
        assert!(toml.contains("[providers.anthropic]"));
        assert!(toml.contains("[providers.openai]"));
        assert!(toml.contains("[providers.openrouter]"));
    }

    #[test]
    fn test_generate_config_no_providers() {
        let answers = InitAnswers {
            providers: vec![],
            default_model: "anthropic/claude-sonnet-4-20250514".into(),
            gateways: vec![],
        };
        let toml = generate_config_toml(&answers);
        assert!(!toml.contains("[providers."));
        assert!(toml.contains("[defaults]"));
    }

    #[test]
    fn test_generate_config_ollama() {
        let answers = InitAnswers {
            providers: vec![ProviderAnswer {
                name: "ollama".into(),
                protocol: "openai".into(),
                base_url: "http://myhost:11434".into(),
                api_key: None,
                api_key_env: None,
                auth: None,
                project_id: None,
                region: None,
            }],
            default_model: "ollama/llama3.1".into(),
            gateways: vec![],
        };
        let toml = generate_config_toml(&answers);
        assert!(toml.contains("[providers.ollama]"));
        assert!(toml.contains("http://myhost:11434"));
        assert!(!toml.contains("api_key"));
    }

    #[test]
    fn test_generate_config_vertex() {
        let answers = InitAnswers {
            providers: vec![ProviderAnswer {
                name: "vertex".into(),
                protocol: "anthropic".into(),
                base_url: "https://europe-west1-aiplatform.googleapis.com".into(),
                api_key: None,
                api_key_env: None,
                auth: Some("vertex".into()),
                project_id: Some("my-gcp-project".into()),
                region: Some("europe-west1".into()),
            }],
            default_model: "vertex/claude-sonnet-4-20250514".into(),
            gateways: vec![],
        };
        let toml = generate_config_toml(&answers);
        assert!(toml.contains("[providers.vertex]"));
        assert!(toml.contains("auth = \"vertex\""));
        assert!(toml.contains("project_id = \"my-gcp-project\""));
        assert!(toml.contains("region = \"europe-west1\""));
    }

    #[test]
    fn test_generate_config_bedrock() {
        let answers = InitAnswers {
            providers: vec![ProviderAnswer {
                name: "bedrock".into(),
                protocol: "anthropic".into(),
                base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
                api_key: None,
                api_key_env: None,
                auth: Some("bedrock".into()),
                project_id: None,
                region: Some("us-east-1".into()),
            }],
            default_model: "bedrock/anthropic.claude-sonnet-4-20250514-v1:0".into(),
            gateways: vec![],
        };
        let toml = generate_config_toml(&answers);
        assert!(toml.contains("[providers.bedrock]"));
        assert!(toml.contains("auth = \"bedrock\""));
        assert!(toml.contains("region = \"us-east-1\""));
        assert!(!toml.contains("project_id"));
    }

    #[test]
    fn test_generate_config_custom_default_model() {
        let answers = InitAnswers {
            providers: vec![ProviderAnswer {
                name: "openai".into(),
                protocol: "openai".into(),
                base_url: "https://api.openai.com".into(),
                api_key: Some("sk-test".into()),
                api_key_env: None,
                auth: None,
                project_id: None,
                region: None,
            }],
            default_model: "openai/gpt-4o".into(),
            gateways: vec![],
        };
        let toml = generate_config_toml(&answers);
        assert!(toml.contains("model = \"openai/gpt-4o\""));
    }

    #[test]
    fn test_generate_config_with_slack_gateway() {
        let answers = InitAnswers {
            providers: vec![],
            default_model: "anthropic/claude-sonnet-4-20250514".into(),
            gateways: vec![GatewayAnswer {
                name: "slack".into(),
                string_fields: vec![
                    ("app_token".into(), "xapp-test-123".into()),
                    ("bot_token".into(), "xoxb-test-456".into()),
                ],
                int_fields: vec![],
            }],
        };
        let toml = generate_config_toml(&answers);
        assert!(toml.contains("[gateway.slack]"));
        assert!(toml.contains("app_token = \"xapp-test-123\""));
        assert!(toml.contains("bot_token = \"xoxb-test-456\""));
    }

    #[test]
    fn test_generate_config_with_discord_gateway() {
        let answers = InitAnswers {
            providers: vec![],
            default_model: "anthropic/claude-sonnet-4-20250514".into(),
            gateways: vec![GatewayAnswer {
                name: "discord".into(),
                string_fields: vec![("bot_token".into(), "discord-token-abc".into())],
                int_fields: vec![],
            }],
        };
        let toml = generate_config_toml(&answers);
        assert!(toml.contains("[gateway.discord]"));
        assert!(toml.contains("bot_token = \"discord-token-abc\""));
    }

    #[test]
    fn test_generate_config_with_webui_gateway() {
        let answers = InitAnswers {
            providers: vec![],
            default_model: "anthropic/claude-sonnet-4-20250514".into(),
            gateways: vec![GatewayAnswer {
                name: "webui".into(),
                string_fields: vec![("host".into(), "0.0.0.0".into())],
                int_fields: vec![("port".into(), 8080)],
            }],
        };
        let toml = generate_config_toml(&answers);
        assert!(toml.contains("[gateway.webui]"));
        assert!(toml.contains("host = \"0.0.0.0\""));
        assert!(toml.contains("port = 8080"));
    }

    #[test]
    fn test_generate_config_with_all_gateways() {
        let answers = InitAnswers {
            providers: vec![],
            default_model: "anthropic/claude-sonnet-4-20250514".into(),
            gateways: vec![
                GatewayAnswer {
                    name: "slack".into(),
                    string_fields: vec![
                        ("app_token".into(), "xapp-1".into()),
                        ("bot_token".into(), "xoxb-1".into()),
                    ],
                    int_fields: vec![],
                },
                GatewayAnswer {
                    name: "discord".into(),
                    string_fields: vec![("bot_token".into(), "discord-1".into())],
                    int_fields: vec![],
                },
                GatewayAnswer {
                    name: "webui".into(),
                    string_fields: vec![("host".into(), "127.0.0.1".into())],
                    int_fields: vec![("port".into(), 3000)],
                },
            ],
        };
        let toml = generate_config_toml(&answers);
        assert!(toml.contains("[gateway.slack]"));
        assert!(toml.contains("[gateway.discord]"));
        assert!(toml.contains("[gateway.webui]"));
    }

    #[test]
    fn test_generated_config_parses_as_valid_toml() {
        let answers = InitAnswers {
            providers: vec![
                ProviderAnswer {
                    name: "anthropic".into(),
                    protocol: "anthropic".into(),
                    base_url: "https://api.anthropic.com".into(),
                    api_key: Some("sk-ant-roundtrip".into()),
                    api_key_env: None,
                    auth: None,
                    project_id: None,
                    region: None,
                },
                ProviderAnswer {
                    name: "openai".into(),
                    protocol: "openai".into(),
                    base_url: "https://api.openai.com".into(),
                    api_key: Some("sk-openai-roundtrip".into()),
                    api_key_env: None,
                    auth: None,
                    project_id: None,
                    region: None,
                },
            ],
            default_model: "anthropic/claude-sonnet-4-20250514".into(),
            gateways: vec![GatewayAnswer {
                name: "webui".into(),
                string_fields: vec![("host".into(), "0.0.0.0".into())],
                int_fields: vec![("port".into(), 8080)],
            }],
        };
        let toml_str = generate_config_toml(&answers);
        let parsed: crate::config::types::AppConfig =
            toml::from_str(&toml_str).expect("Generated TOML should parse as AppConfig");
        assert_eq!(parsed.defaults.model, "anthropic/claude-sonnet-4-20250514");
        assert_eq!(parsed.defaults.max_tokens, 8192);
        let anthropic = parsed
            .providers
            .get("anthropic")
            .expect("anthropic provider");
        assert_eq!(anthropic.api_key.as_deref(), Some("sk-ant-roundtrip"));
        let openai = parsed.providers.get("openai").expect("openai provider");
        assert_eq!(openai.api_key.as_deref(), Some("sk-openai-roundtrip"));
        let webui = parsed.gateway.webui.expect("webui gateway");
        assert_eq!(webui.host(), "0.0.0.0");
        assert_eq!(webui.port(), 8080);
    }

    // -----------------------------------------------------------------------
    // write_config tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_write_config_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        write_config(&path, "test content").unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "test content");
    }

    #[test]
    fn test_write_config_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("deep").join("config.toml");
        write_config(&path, "nested content").unwrap();
        assert!(path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_write_config_unix_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        write_config(&path, "secret").unwrap();
        let perms = std::fs::metadata(&path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
    }

    // -----------------------------------------------------------------------
    // run_init flow tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_run_init_happy_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut prompter = MockPrompter::new()
            .with_multi_selects(vec![vec![0]]) // Anthropic
            .with_selects(vec![
                0, // auth method: API Key
                0, // first model
            ])
            .with_passwords(vec!["sk-ant-happy"])
            .with_confirms(vec![false]); // no gateways
        run_init(&mut prompter, &path).unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("sk-ant-happy"));
        assert!(content.contains("[providers.anthropic]"));
        assert!(prompter.outputs.iter().any(|o| o.contains("Try it out")));
    }

    #[test]
    fn test_run_init_multiple_providers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut prompter = MockPrompter::new()
            .with_multi_selects(vec![vec![0, 1]]) // Anthropic + OpenAI
            .with_selects(vec![
                0, // auth method: API Key (Anthropic)
                0, // first model
            ])
            .with_passwords(vec!["sk-ant-test", "sk-openai-test"])
            .with_confirms(vec![false]);
        run_init(&mut prompter, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[providers.anthropic]"));
        assert!(content.contains("sk-ant-test"));
        assert!(content.contains("[providers.openai]"));
        assert!(content.contains("sk-openai-test"));
    }

    #[test]
    fn test_run_init_ollama() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut prompter = MockPrompter::new()
            .with_multi_selects(vec![vec![3]]) // Ollama
            .with_inputs(vec!["http://myhost:11434"]) // base URL
            .with_selects(vec![0])
            .with_confirms(vec![false]);
        run_init(&mut prompter, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[providers.ollama]"));
        assert!(content.contains("http://myhost:11434"));
        assert!(content.contains("model = \"ollama/llama3.3\""));
    }

    #[test]
    fn test_run_init_vertex() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut prompter = MockPrompter::new()
            .with_multi_selects(vec![vec![4]]) // Vertex AI
            .with_inputs(vec!["europe-west1", "my-gcp-project"]) // region, project_id
            .with_selects(vec![0])
            .with_confirms(vec![false]);
        run_init(&mut prompter, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[providers.vertex]"));
        assert!(content.contains("auth = \"vertex\""));
        assert!(content.contains("project_id = \"my-gcp-project\""));
        assert!(content.contains("region = \"europe-west1\""));
        assert!(content.contains("europe-west1-aiplatform.googleapis.com"));
    }

    #[test]
    fn test_run_init_with_slack_gateway() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut prompter = MockPrompter::new()
            .with_multi_selects(vec![vec![0], vec![0]]) // Anthropic; Slack
            .with_selects(vec![
                0, // auth method: API Key
                0, // first model
            ])
            .with_passwords(vec!["sk-ant-test", "xapp-test-123", "xoxb-test-456"])
            .with_confirms(vec![true]); // yes gateways
        run_init(&mut prompter, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[gateway.slack]"));
        assert!(content.contains("app_token = \"xapp-test-123\""));
        assert!(content.contains("bot_token = \"xoxb-test-456\""));
    }

    #[test]
    fn test_run_init_with_webui_gateway() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut prompter = MockPrompter::new()
            .with_multi_selects(vec![vec![0], vec![2]]) // Anthropic; WebUI
            .with_selects(vec![
                0, // auth method: API Key
                0, // first model
            ])
            .with_passwords(vec!["sk-ant-test"])
            .with_inputs(vec!["0.0.0.0", "8080"]) // host, port
            .with_confirms(vec![true]); // yes gateways
        run_init(&mut prompter, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[gateway.webui]"));
        assert!(content.contains("host = \"0.0.0.0\""));
        assert!(content.contains("port = 8080"));
    }

    #[test]
    fn test_run_init_no_providers_skips_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut prompter = MockPrompter::new().with_multi_selects(vec![vec![]]);
        run_init(&mut prompter, &path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_run_init_existing_config_abort() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "existing").unwrap();
        let mut prompter = MockPrompter::new().with_confirms(vec![false]); // decline overwrite
        run_init(&mut prompter, &path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "existing");
    }

    #[test]
    fn test_run_init_existing_config_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "old content").unwrap();
        let mut prompter = MockPrompter::new()
            .with_confirms(vec![true, false]) // overwrite + no gateways
            .with_multi_selects(vec![vec![0]]) // Anthropic
            .with_selects(vec![
                0, // auth method: API Key
                0, // first model
            ])
            .with_passwords(vec!["sk-ant-new"]);
        run_init(&mut prompter, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("sk-ant-new"));
        assert!(!content.contains("old content"));
    }

    #[test]
    fn test_run_init_with_setup_token() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut prompter = MockPrompter::new()
            .with_multi_selects(vec![vec![0]]) // Anthropic
            .with_selects(vec![
                1, // auth method: setup-token (index 1)
                0, // first model
            ])
            .with_passwords(vec!["sk-ant-oat01-test-token-123"])
            .with_confirms(vec![false]); // no gateways
        run_init(&mut prompter, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[providers.anthropic]"));
        assert!(content.contains("api_key = \"sk-ant-oat01-test-token-123\""));
        assert!(!content.contains("api_key_env"));
    }

    #[test]
    fn test_generate_config_setup_token() {
        let answers = InitAnswers {
            providers: vec![ProviderAnswer {
                name: "anthropic".into(),
                protocol: "anthropic".into(),
                base_url: "https://api.anthropic.com".into(),
                api_key: Some("sk-ant-oat01-test-token-123".into()),
                api_key_env: None,
                auth: None,
                project_id: None,
                region: None,
            }],
            default_model: "anthropic/claude-opus-4-6".into(),
            gateways: vec![],
        };
        let toml = generate_config_toml(&answers);
        assert!(toml.contains("[providers.anthropic]"));
        assert!(toml.contains("api_key = \"sk-ant-oat01-test-token-123\""));
    }

    #[test]
    fn test_run_init_api_key_empty_uses_env() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut prompter = MockPrompter::new()
            .with_multi_selects(vec![vec![0]]) // Anthropic
            .with_selects(vec![
                0, // auth method: API Key
                0, // first model
            ])
            .with_passwords(vec![""]) // empty key
            .with_confirms(vec![false]);
        run_init(&mut prompter, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("api_key_env = \"ANTHROPIC_API_KEY\""));
        assert!(!content.contains("api_key = \"\""));
    }
}
