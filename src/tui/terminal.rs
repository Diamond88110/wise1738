use std::{fs, io, time::Duration};

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

use printpdf::*;

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
    last_results: Vec<ScanResult>,
}

impl App {
    fn new() -> Self {
        Self {
            state: UiState::Idle,
            command: String::new(),
            events: Vec::new(),
            open: Vec::new(),
            closed: Vec::new(),
            scroll: 0,
            last_results: Vec::new(),
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
    app.event("Commands: scan | export json | export pdf | exit");

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

    match parts.as_slice() {
        ["exit"] | ["q"] => {
            app.event("Exit requested");
            app.state = UiState::ExitPending;
        }
        ["export", "json"] => export_json(app),
        ["export", "pdf"] => export_pdf(app),
        ["scan", ..] => handle_scan(parts, app),
        _ => app.event("Unknown command"),
    }
}

// =======================
// SCAN HANDLER
// =======================
fn handle_scan(parts: Vec<&str>, app: &mut App) {
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
    app.last_results.clear();

    app.event(format!("Scanning {}", host));

    let results = engine::run(host, ports);

    for r in &results {
        let mut service = String::new();
        if r.service != "unknown" {
            service.push_str(r.service);
        }
        if let Some(os) = r.os_hint {
            if !service.is_empty() {
                service.push(' ');
            }
            service.push_str(&format!("[{}]", os));
        }

        match r.status {
            PortStatus::Open => {
                app.open.push(format!("{:<5} OPEN   {}", r.port, service));
            }
            _ => {
                app.closed.push(format!("{:<5} CLOSED {}", r.port, service));
            }
        }
    }

    app.last_results = results;
    app.event("Scan finished");
}

// =======================
// EXPORT JSON
// =======================
fn export_json(app: &mut App) {
    if app.last_results.is_empty() {
        app.event("Nothing to export");
        return;
    }

    fs::create_dir_all("export").ok();
    let file = format!("export/scan_{}.json", Local::now().format("%Y%m%d_%H%M%S"));

    let mut json = String::from("{\"results\":[");
    for (i, r) in app.last_results.iter().enumerate() {
        json.push_str(&format!(
            "{{\"port\":{},\"status\":\"{:?}\",\"service\":\"{}\",\"os\":{}}}",
            r.port,
            r.status,
            r.service,
            match r.os_hint {
                Some(os) => format!("\"{}\"", os),
                None => "null".into(),
            }
        ));
        if i + 1 < app.last_results.len() {
            json.push(',');
        }
    }
    json.push_str("]}");

    match fs::write(&file, json) {
        Ok(_) => app.event(format!("Exported JSON → {}", file)),
        Err(_) => app.event("JSON export failed"),
    }
}

// =======================
// EXPORT PDF (PAGINATED)
// =======================
fn export_pdf(app: &mut App) {
    if app.last_results.is_empty() {
        app.event("Nothing to export");
        return;
    }

    fs::create_dir_all("export").ok();
    let file_path = format!("export/scan_{}.pdf", Local::now().format("%Y%m%d_%H%M%S"));

    let (doc, mut page, mut layer) =
        PdfDocument::new("WISE1738 Scan Report", Mm(210.0), Mm(297.0), "Layer");

    let font = doc.add_builtin_font(BuiltinFont::Courier).unwrap();
    let mut y = Mm(280.0);
    let line_h = Mm(6.0);

    let mut cur_layer = doc.get_page(page).get_layer(layer);
    cur_layer.use_text("WISE1738 Scan Report", 14.0, Mm(10.0), y, &font);
    y -= Mm(12.0);

    for r in &app.last_results {
        if y.0 < 20.0 {
            let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Layer");
            page = p;
            layer = l;
            cur_layer = doc.get_page(page).get_layer(layer);
            y = Mm(280.0);
        }

        let line = format!(
            "Port {:<5} {:<8} {} {}",
            r.port,
            format!("{:?}", r.status),
            r.service,
            r.os_hint.unwrap_or("")
        );

        cur_layer.use_text(line, 10.0, Mm(10.0), y, &font);
        y -= line_h;
    }

    let mut file = std::io::BufWriter::new(std::fs::File::create(&file_path).unwrap());
    doc.save(&mut file).unwrap();

    app.event(format!("Exported PDF → {}", file_path));
}

// =======================
// PORT PARSER
// =======================
fn parse_ports(raw: &str) -> Option<Ports> {
    if raw.contains(',') {
        Some(Ports::multiple(
            raw.split(',').map(|p| p.parse().ok()).collect::<Option<Vec<_>>>()?,
        ))
    } else if raw.contains('-') {
        let p: Vec<&str> = raw.split('-').collect();
        Some(Ports::range(p[0].parse().ok()?, p[1].parse().ok()?))
    } else {
        Some(Ports::single(raw.parse().ok()?))
    }
}

// =======================
// UI RENDER (GRID + SCROLL)
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

    f.render_widget(
        Paragraph::new(" WISE1738 | STATE: IDLE ")
            .style(Style::default().fg(Color::Gray)),
        layout[0],
    );

    f.render_widget(
        Paragraph::new(format!("> {}", app.command))
            .block(Block::default().title(" COMMAND ").borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan)),
        layout[1],
    );

    let area = layout[2];
    let mut lines: Vec<Line> = Vec::new();

    if !app.open.is_empty() {
        lines.push(Line::from(Span::styled(
            "[ OPEN PORTS ]",
            Style::default().fg(Color::Green),
        )));
        for l in &app.open {
            lines.push(Line::from(Span::styled(l, Style::default().fg(Color::Green))));
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

    f.render_widget(
        Paragraph::new(app.events.join("\n"))
            .block(Block::default().title(" EVENTS ").borders(Borders::ALL)),
        layout[3],
    );
}

