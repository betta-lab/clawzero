pub mod app;
pub mod event;
pub mod markdown;
pub mod ui;
pub mod widgets;

use std::io;
use std::time::Duration;

use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::agent::event::AgentEvent;
use crate::agent::factory::AgentFactory;
use crate::agent::r#loop::Agent;
use crate::cli::tui::app::{App, AppAction, AppMode};
use crate::cli::tui::event::{EventLoop, TuiEvent};
use crate::session::store::SessionStore;

/// Fixed viewport height. Large enough for: streaming (12) + tool card (2) + status (1) + input (1).
const VIEWPORT_HEIGHT: u16 = 16;

/// Setup terminal for inline TUI mode (Viewport::Inline with fixed height).
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let options = ratatui::TerminalOptions {
        viewport: ratatui::Viewport::Inline(VIEWPORT_HEIGHT),
    };
    let terminal = Terminal::with_options(backend, options)?;
    Ok(terminal)
}

/// Restore terminal to normal mode.
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = terminal.show_cursor();
}

/// Flush pending insert lines via insert_before, then clear the buffer.
fn flush_inserts(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    lines: &mut Vec<Line<'static>>,
) -> Result<()> {
    if !lines.is_empty() {
        let drained: Vec<Line<'static>> = std::mem::take(lines);
        terminal.insert_before(drained.len() as u16, |buf| {
            let paragraph = Paragraph::new(drained);
            ratatui::widgets::Widget::render(paragraph, buf.area, buf);
        })?;
    }
    Ok(())
}

/// Print the inline header (not in viewport, just normal terminal output).
fn print_header(model: &str, session_id: Option<&str>) {
    let session_part = session_id
        .map(|s| {
            if s.len() > 12 {
                format!("  session: {}...", &s[..12])
            } else {
                format!("  session: {s}")
            }
        })
        .unwrap_or_default();
    println!("clawzero ({model}){session_part}");
    println!();
}

/// Spawn an agent task that sends events back via mpsc channel.
/// Returns a JoinHandle that yields the agent back.
fn spawn_agent_task(
    mut agent: Agent,
    input: String,
    event_tx: mpsc::Sender<AgentEvent>,
) -> JoinHandle<Agent> {
    tokio::spawn(async move {
        agent
            .run(input, |ev| {
                let _ = event_tx.try_send(ev.clone());
            })
            .await;
        agent
    })
}

/// Run the TUI REPL (interactive mode) with inline viewport.
pub async fn run_tui_repl(
    factory: &AgentFactory,
    session_store: Option<&SessionStore>,
) -> Result<()> {
    let mut agent = match session_store {
        Some(store) => {
            if let Ok(writer) = store.create_session(factory.model()) {
                factory.create_with_session(writer)
            } else {
                factory.create()
            }
        }
        None => factory.create(),
    };

    let model = factory.model().to_string();
    let session_id = agent.session_id().map(|s| s.to_string());
    let mut app = App::new(model.clone(), session_id.clone());

    // Print header before entering raw mode
    print_header(&model, session_id.as_deref());

    let mut terminal = setup_terminal()?;

    // Install panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        original_hook(info);
    }));

    let (event_tx, event_rx) = mpsc::channel::<AgentEvent>(256);
    let mut event_loop = EventLoop::new(event_rx, Duration::from_millis(150));
    let mut agent_handle: Option<JoinHandle<Agent>> = None;

    // Initial draw
    terminal.draw(|frame| ui::draw_live(frame, &app))?;

    loop {
        let event = event_loop.next().await;

        match event {
            Some(TuiEvent::Terminal(crossterm::event::Event::Key(key))) => {
                if let Some(action) = app.handle_key_event(key) {
                    match action {
                        AppAction::Quit => break,
                        AppAction::Submit(text) => {
                            // Flush user message lines
                            flush_inserts(&mut terminal, &mut app.pending_inserts)?;

                            let handle = spawn_agent_task(agent, text, event_tx.clone());
                            agent_handle = Some(handle);
                            // agent will be reclaimed when Done/Error is received
                            agent = factory.create(); // placeholder
                        }
                    }
                }
            }
            Some(TuiEvent::Terminal(crossterm::event::Event::Resize(_, _))) => {
                // Terminal resize — just redraw
            }
            Some(TuiEvent::Terminal(_)) => {}
            Some(TuiEvent::Agent(event)) => {
                let is_done = matches!(event, AgentEvent::Done { .. } | AgentEvent::Error(_));
                app.handle_agent_event(&event);

                // Flush confirmed content
                flush_inserts(&mut terminal, &mut app.pending_inserts)?;

                if is_done {
                    // Reclaim agent from the spawn handle
                    if let Some(handle) = agent_handle.take() {
                        match handle.await {
                            Ok(returned_agent) => agent = returned_agent,
                            Err(_) => {
                                agent = factory.create();
                            }
                        }
                    }
                }
            }
            Some(TuiEvent::Tick) => {
                if app.mode != AppMode::Idle {
                    app.tick();
                }
            }
            None => break,
        }

        terminal.draw(|frame| ui::draw_live(frame, &app))?;
    }

    restore_terminal(&mut terminal);
    println!();
    Ok(())
}

/// Run the TUI in one-shot mode (prompt given, display result, then exit).
pub async fn run_tui_oneshot(
    factory: &AgentFactory,
    session_store: Option<&SessionStore>,
    prompt: String,
) -> Result<()> {
    let agent = match session_store {
        Some(store) => {
            if let Ok(writer) = store.create_session(factory.model()) {
                factory.create_with_session(writer)
            } else {
                factory.create()
            }
        }
        None => factory.create(),
    };

    let model = factory.model().to_string();
    let session_id = agent.session_id().map(|s| s.to_string());
    let mut app = App::new(model.clone(), session_id.clone());

    // Print header before entering raw mode
    print_header(&model, session_id.as_deref());

    // Push user message to pending_inserts
    app.push_user_message(&prompt);
    app.mode = AppMode::Thinking;

    let mut terminal = setup_terminal()?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        original_hook(info);
    }));

    // Flush user message immediately
    flush_inserts(&mut terminal, &mut app.pending_inserts)?;

    // Submit immediately
    let (event_tx, event_rx) = mpsc::channel::<AgentEvent>(256);
    let mut event_loop = EventLoop::new(event_rx, Duration::from_millis(150));
    let _agent_handle = spawn_agent_task(agent, prompt, event_tx);

    terminal.draw(|frame| ui::draw_live(frame, &app))?;

    loop {
        let event = event_loop.next().await;

        match event {
            Some(TuiEvent::Terminal(crossterm::event::Event::Key(key))) => {
                if let Some(AppAction::Quit) = app.handle_key_event(key) {
                    break;
                }
            }
            Some(TuiEvent::Agent(event)) => {
                let is_done = matches!(event, AgentEvent::Done { .. } | AgentEvent::Error(_));
                app.handle_agent_event(&event);

                // Flush confirmed content
                flush_inserts(&mut terminal, &mut app.pending_inserts)?;

                if is_done {
                    // One-shot: final draw then exit
                    terminal.draw(|frame| ui::draw_live(frame, &app))?;
                    break;
                }
            }
            Some(TuiEvent::Tick) => {
                if app.mode != AppMode::Idle {
                    app.tick();
                }
            }
            Some(TuiEvent::Terminal(_)) => {}
            None => break,
        }

        terminal.draw(|frame| ui::draw_live(frame, &app))?;
    }

    restore_terminal(&mut terminal);
    println!();
    Ok(())
}
