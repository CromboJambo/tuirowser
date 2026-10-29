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

/// Commands that can be sent to the browser from external sources (HTTP API, etc.)
#[derive(Debug, Clone)]
pub enum BrowserCommand {
    Navigate(String),
    Reload,
    Quit,
}

/// Internal command representation for unified handling
#[derive(Debug, Clone)]
enum Command {
    Enter,
    Escape,
    Reload,
    Quit,
    GoUrlInput,
    Char(char),
}

/// Input modes for the browser.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    /// Normal navigation mode - keys trigger commands.
    Normal,
    /// URL input mode - keystrokes build a URL string.
    UrlInput,
}

/// Application state passed to the UI loop.
pub struct AppState {
    /// Current URL being displayed.
    pub url: String,
    /// Markdown content to display.
    pub content: String,
    /// Whether the app should continue running.
    pub running: bool,
    /// Current input mode.
    pub mode: InputMode,
    /// Buffer for URL being typed in UrlInput mode.
    pub url_input: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            url: "about:blank".to_string(),
            content: "".to_string(),
            running: true,
            mode: InputMode::Normal,
            url_input: String::new(),
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

/// Async version that accepts commands from a channel (for HTTP/API control).
pub async fn run_app_with_commands(mut state: AppState, mut rx: tokio::sync::mpsc::Receiver<BrowserCommand>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop - handle both keyboard events and channel commands
    while state.running {
        terminal.draw(|frame| render_frame(frame, &state))?;

        // Check for channel command (non-blocking)
        match rx.try_recv() {
            Ok(BrowserCommand::Navigate(url)) => {
                // Switch to URL input mode and fill in the URL
                state.mode = InputMode::UrlInput;
                state.url_input = url.clone();
                // Submit immediately by simulating Enter
                handle_command(&mut state, Command::Enter);
            }
            Ok(BrowserCommand::Reload) => {
                handle_command(&mut state, Command::Reload);
            }
            Ok(BrowserCommand::Quit) => {
                state.running = false;
            }
            Err(_) => {
                // No command from channel, handle keyboard events (blocking)
                handle_input(&mut state)?;
            }
        }
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

    // URL bar - shows current URL or input buffer depending on mode
    let display_url = match state.mode {
        InputMode::UrlInput => format!("> {}", state.url_input),
        InputMode::Normal => state.url.clone(),
    };

    let url_bar = Paragraph::new(Span::styled(
        display_url,
        Style::default()
            .fg(if state.mode == InputMode::UrlInput { Color::Yellow } else { Color::Cyan })
            .add_modifier(Modifier::BOLD),
    ))
    .block(Block::default().borders(Borders::ALL).title("termview"));
    frame.render_widget(url_bar, chunks[0]);

    // Content area
    let content = Paragraph::new(state.content.as_str());
    frame.render_widget(content, chunks[1]);

    // Status bar - mode indicator and available commands
    let status_line = match state.mode {
        InputMode::UrlInput => Line::from(vec![
            Span::styled("[ESC] cancel [Enter] go ", Style::default().fg(Color::Yellow)),
        ]),
        InputMode::Normal => Line::from(vec![
            Span::styled("[g]o [q]uit [r]eload ", Style::default().fg(Color::Gray)),
        ]),
    };
    frame.render_widget(status_line, chunks[2]);
}

/// Handle keyboard input with modal state machine.
fn handle_input(state: &mut AppState) -> Result<()> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key_event) = event::read()? {
            if key_event.kind == KeyEventKind::Press {
                match state.mode {
                    InputMode::UrlInput => handle_url_input(state, key_event)?,
                    InputMode::Normal => handle_normal_input(state, key_event)?,
                }
            }
        }
    }
    Ok(())
}

/// Handle input while in URL entry mode.
fn handle_url_input(state: &mut AppState, key_event: crossterm::event::KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Enter => {
            // Submit URL and navigate
            if !state.url_input.is_empty() {
                let url = state.url_input.clone();
                state.url = url.clone();
                state.content = format!("Loading: {}", url);
                // TODO: Actual fetch and parse logic here
            }
            state.mode = InputMode::Normal;
            state.url_input.clear();
        }
        KeyCode::Esc => {
            // Cancel URL input, return to normal mode
            state.mode = InputMode::Normal;
            state.url_input.clear();
        }
        KeyCode::Backspace => {
            state.url_input.pop();
        }
        KeyCode::Char(c) => {
            state.url_input.push(c);
        }
        _ => {}
    }
    Ok(())
}

/// Handle input while in normal navigation mode.
fn handle_normal_input(state: &mut AppState, key_event: crossterm::event::KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Char('q') => {
            state.running = false;
        }
        KeyCode::Char('r') => {
            // Reload current page
            if !state.url.is_empty() && state.url != "about:blank" {
                let url = state.url.clone();
                state.content = format!("Reloading: {}", url);
                // TODO: Actual fetch logic here
            }
        }
        KeyCode::Char('g') => {
            // Enter URL input mode
            state.mode = InputMode::UrlInput;
            state.url_input.clear();
        }
        _ => {}
    }
    Ok(())
}

/// Handle a command (from keyboard or channel).
fn handle_command(state: &mut AppState, cmd: Command) {
    match state.mode {
        InputMode::UrlInput => handle_url_input_command(state, cmd),
        InputMode::Normal => handle_normal_input_command(state, cmd),
    }
}

/// Handle input while in URL entry mode.
fn handle_url_input_command(state: &mut AppState, cmd: Command) {
    match cmd {
        Command::Enter => {
            // Submit URL and navigate
            if !state.url_input.is_empty() {
                let url = state.url_input.clone();
                state.url = url.clone();
                state.content = format!("Loading: {}", url);
                // TODO: Actual fetch and parse logic here
            }
            state.mode = InputMode::Normal;
            state.url_input.clear();
        }
        Command::Escape => {
            // Cancel URL input
            state.url_input = String::new();
            state.mode = InputMode::Normal;
        }
        Command::Char(c) => {
            // Append character to URL input
            state.url_input.push(c);
        }
        _ => {}
    }
}

/// Handle input while in normal mode.
fn handle_normal_input_command(state: &mut AppState, cmd: Command) {
    match cmd {
        Command::GoUrlInput => {
            // Enter URL input mode
            state.mode = InputMode::UrlInput;
            state.url_input = String::new();
        }
        Command::Reload => {
            // Reload current page
            if !state.url.is_empty() && state.url != "about:blank" {
                let url = state.url.clone();
                state.content = format!("Reloading: {}", url);
                // TODO: Actual fetch logic here
            }
        }
        Command::Quit => {
            state.running = false;
        }
        _ => {}
    }
}