use std::time::Duration;

use crossterm::event::{Event, EventStream};
use tokio::sync::mpsc;

use crate::agent::event::AgentEvent;

/// Unified event type for the TUI event loop.
#[derive(Debug)]
pub enum TuiEvent {
    /// Terminal input event (key press, resize, etc.)
    Terminal(Event),
    /// Agent event received via channel
    Agent(AgentEvent),
    /// Periodic tick for animations
    Tick,
}

/// Event loop that multiplexes terminal events, agent events, and ticks.
pub struct EventLoop {
    agent_rx: mpsc::Receiver<AgentEvent>,
    tick_interval: Duration,
}

impl EventLoop {
    pub fn new(agent_rx: mpsc::Receiver<AgentEvent>, tick_interval: Duration) -> Self {
        Self {
            agent_rx,
            tick_interval,
        }
    }

    /// Wait for the next event, returning it.
    pub async fn next(&mut self) -> Option<TuiEvent> {
        use futures_util::StreamExt;

        let mut term_stream = EventStream::new();
        let tick_sleep = tokio::time::sleep(self.tick_interval);
        tokio::pin!(tick_sleep);

        tokio::select! {
            biased;

            Some(event) = self.agent_rx.recv() => {
                Some(TuiEvent::Agent(event))
            }
            maybe_event = term_stream.next() => {
                match maybe_event {
                    Some(Ok(event)) => Some(TuiEvent::Terminal(event)),
                    Some(Err(_)) => None,
                    None => None,
                }
            }
            () = &mut tick_sleep => {
                Some(TuiEvent::Tick)
            }
        }
    }
}
