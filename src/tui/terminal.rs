use std::{io, time::Duration};

use chrono::Local;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use crate::core::{
    engine,
    ports::Ports,
    scanner::{PortStatus, ScanResult},
};

// =======================
// UI STATE
// =======================
enum UiState {
    Idle,
    ExitPending,
}

// =======================
// APP STATE
// =======================
struct App {
    state: UiState,
    command: String,
    events: Vec<String>,
    open: Vec<String>,
    closed: Vec<String>,
    scroll: usize,
}

impl App {
    fn new() -> Self {
        Self {
            state: UiState::Idle,
            command: String::new(),
            events: vec![],
            open: Vec::new(),
            closed: Vec::new(),
            scroll: 0,
        }
    }

    fn event(&mut self, msg: impl Into<String>) {
        let ts = Local::now().format("%H:%M:%S");
        self.events.push(format!("[{}] {}", ts, msg.into()));
        if self.events.len() > 6 {
            self.events.remove(0);
        }
    }
}

// =======================
// ENTRY
// =======================
pub fn run() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.event("WISE1738 ready");

    let res = event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

// =======================
// EVENT LOOP
// =======================
fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| draw_ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(c) => app.command.push(c),
                    KeyCode::Backspace => {
                        app.command.pop();
                    }
                    KeyCode::Up => {
                        if app.scroll > 0 {
                            app.scroll -= 1;
                        }
                    }
                    KeyCode::Down => {
                        app.scroll = app.scroll.saturating_add(1);
                    }
                    KeyCode::Enter => {
                        let cmd = app.command.trim().to_string();
                        app.command.clear();

                        match app.state {
                            UiState::ExitPending => return Ok(()),
                            UiState::Idle => handle_command(&cmd, app),
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

// =======================
// COMMAND HANDLER
// =======================
fn handle_command(cmd: &str, app: &mut App) {
    if cmd.is_empty() {
        return;
    }

    let parts: Vec<&str> = cmd.split_whitespace().collect();

    match parts[0] {
        "exit" | "q" => {
            app.event("Exit requested");
            app.state = UiState::ExitPending;
        }

        "scan" => {
            if parts.len() < 2 || parts.len() > 3 {
                app.event("Usage: scan <ip|domain> [ports]");
                return;
            }

            let host = parts[1];
            let ports = if parts.len() == 3 {

match parse_ports(parts[2]) {
                    Some(p) => p,
                    None => {
                        app.event("Invalid port format");
                        return;
                    }
                }
            } else {
                Ports::all()
            };

            app.open.clear();
            app.closed.clear();
            app.scroll = 0;

            app.event(format!("CMD: scan {} {}", host, parts.get(2).unwrap_or(&"")));
            app.event(format!("Scanning {}", host));

            let results: Vec<ScanResult> = engine::run(host, ports);

            for r in results {
                let service = if r.service == "unknown" { "" } else { r.service };

                match r.status {
                    PortStatus::Open => {
                        app.open.push(format!(
                            "{:<5} {:<6} {}",
                            r.port, "OPEN", service
                        ));
                    }
                    PortStatus::Closed | PortStatus::Filtered => {
                        app.closed.push(format!(
                            "{:<5} {:<6} {}",
                            r.port, "CLOSED", service
                        ));
                    }
                }
            }

            app.event("Scan finished");
        }

        _ => app.event("Unknown command"),
    }
}

// =======================
// PORT PARSER
// =======================
fn parse_ports(raw: &str) -> Option<Ports> {
    if raw.contains(',') {
        let mut list = Vec::new();
        for p in raw.split(',') {
            list.push(p.parse().ok()?);
        }
        Some(Ports::multiple(list))
    } else if raw.contains('-') {
        let p: Vec<&str> = raw.split('-').collect();
        Some(Ports::range(p[0].parse().ok()?, p[1].parse().ok()?))
    } else {
        Some(Ports::single(raw.parse().ok()?))
    }
}

// =======================
// UI RENDER
// =======================
fn draw_ui(f: &mut ratatui::Frame, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(6),
        ])
        .split(f.size());

    // STATUS
    f.render_widget(
        Paragraph::new(" WISE1738 | STATE: IDLE ")
            .style(Style::default().fg(Color::Gray)),
        layout[0],
    );

    // COMMAND
    f.render_widget(
        Paragraph::new(format!("> {}", app.command))
            .block(Block::default().title(" COMMAND ").borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan)),
        layout[1],
    );

    // OUTPUT
    let area = layout[2];
    let mut lines: Vec<Line> = Vec::new();

    if !app.open.is_empty() {
        lines.push(Line::from(Span::styled(
            "[ OPEN PORTS ]",
            Style::default().fg(Color::Green),
        )));
        for l in &app.open {
            lines.push(Line::from(Span::styled(
                l.clone(),
                Style::default().fg(Color::Green),
            )));
        }
        lines.push(Line::from(""));
    }

    if !app.closed.is_empty() {
        lines.push(Line::from(Span::styled(
            "[ CLOSED PORTS ]",
            Style::default().fg(Color::DarkGray),
        )));

        let col_width = 28;
        let cols = (area.width as usize / col_width).max(1);
        let rows = (app.closed.len() + cols - 1) / cols;

        for r in 0..rows {
            let mut spans = Vec::new();
            for c in 0..cols {
                let i = r + c * rows;
                if let Some(item) = app.closed.get(i) {
                    spans.push(Span::styled(
                        format!("{:<width$}", item, width = col_width),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            }
            lines.push(Line::from(spans));
        }
    }

let visible = area.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(visible);
    let start = app.scroll.min(max_scroll);
    let end = (start + visible).min(lines.len());

    f.render_widget(
        Paragraph::new(lines[start..end].to_vec())
            .block(Block::default().title(" SCAN OUTPUT (↑ ↓) ").borders(Borders::ALL)),
        area,
    );

    // EVENTS
    f.render_widget(
        Paragraph::new(app.events.join("\n"))
            .block(Block::default().title(" EVENTS ").borders(Borders::ALL)),
        layout[3],
    );
}

