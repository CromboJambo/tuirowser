//! Ratatui-based terminal UI for the markdown browser.
//!
//! This crate handles all terminal rendering, keyboard input, and user interaction.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

/// Application state passed to the UI loop.
pub struct AppState {
    /// Current URL being displayed.
    pub url: String,
    /// Markdown content to display.
    pub content: String,
    /// Whether the app should continue running.
    pub running: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            url: "about:blank".to_string(),
            content: "".to_string(),
            running: true,
        }
    }
}

/// Run the main UI loop.
pub fn run_app(mut state: AppState) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    while state.running {
        terminal.draw(|frame| render_frame(frame, &state))?;
        handle_input(&mut state)?;
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}

/// Render a single frame of the UI.
fn render_frame(frame: &mut ratatui::Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header (URL bar)
            Constraint::Min(0),     // Content area
            Constraint::Length(1),  // Footer (status bar)
        ])
        .split(frame.area());

    // URL bar
    let url_bar = Paragraph::new(Span::styled(
        &state.url,
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ))
    .block(Block::default().borders(Borders::ALL).title("termview"));
    frame.render_widget(url_bar, chunks[0]);

    // Content area
    let content = Paragraph::new(state.content.as_str());
    frame.render_widget(content, chunks[1]);

    // Status bar
    let status = Line::from(vec![
        Span::styled("[q]uit [r]eload [g]o ", Style::default().fg(Color::Gray)),
    ]);
    frame.render_widget(status, chunks[2]);
}

/// Handle keyboard input.
fn handle_input(state: &mut AppState) -> Result<()> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key_event) = event::read()? {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    KeyCode::Char('q') => {
                        state.running = false;
                    }
                    KeyCode::Char('r') => {
                        // Reload current page
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}