use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Wrap},
    Terminal,
};
use std::io;
use tokio::sync::mpsc;

const BG:      Color = Color::Rgb(0x11, 0x11, 0x11);
const SEP:     Color = Color::Rgb(0x2a, 0x2a, 0x2a);
const DIM:     Color = Color::Rgb(0x33, 0x33, 0x33);
const MUTED:   Color = Color::Rgb(0x55, 0x55, 0x55);
const MOAI_FG: Color = Color::Rgb(0x88, 0x88, 0x88);
const USER_FG: Color = Color::Rgb(0xf0, 0xf0, 0xf0);

const LOGO: &str = "\
███╗   ███╗ ██████╗  █████╗ ██╗
████╗ ████║██╔═══██╗██╔══██╗██║
██╔████╔██║██║   ██║███████║██║
██║╚██╔╝██║██║   ██║██╔══██║██║
██║ ╚═╝ ██║╚██████╔╝██║  ██║██║
╚═╝     ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝";

const MODEL: &str = "qwen3-coder:30b";
const OLLAMA: &str = "http://localhost:11434/api/chat";

// tokens the streaming task sends back to the main thread
enum Token {
    Delta(String),  // a chunk of text arrived
    Done,           // stream finished
    Err(String),    // something went wrong
}

enum Speaker { Logo, User, Moai }
struct Message { speaker: Speaker, text: String }

struct App {
    messages:      Vec<Message>,
    input:         String,
    scroll:        u16,
    cc:            bool,
    streaming:     bool,
    pinned_bottom: bool, // auto-follow bottom when true
    tick:          u64,  // frame counter for animation
}

impl App {
    fn new() -> Self {
        Self {
            messages:      vec![Message { speaker: Speaker::Logo, text: LOGO.to_string() }],
            input:         String::new(),
            scroll:        0,
            cc:            false,
            streaming:     false,
            pinned_bottom: true,
            tick:          0,
        }
    }

    fn submit(&mut self) -> Option<String> {
        let t = self.input.trim().to_string();
        if t.is_empty() || self.streaming { return None; }
        self.messages.push(Message { speaker: Speaker::User, text: t.clone() });
        self.messages.push(Message { speaker: Speaker::Moai, text: String::new() });
        self.input.clear();
        self.pinned_bottom = true; // snap to bottom on new message
        self.streaming = true;
        Some(t)
    }

    fn append_delta(&mut self, delta: &str) {
        if let Some(msg) = self.messages.iter_mut().rev().find(|m| matches!(m.speaker, Speaker::Moai)) {
            msg.text.push_str(delta);
        }
        // only auto-scroll if user hasn't manually scrolled up
    }

    fn finish_stream(&mut self) {
        self.streaming = false;
        // if moai said nothing, show a fallback
        if let Some(msg) = self.messages.iter_mut().rev().find(|m| matches!(m.speaker, Speaker::Moai)) {
            if msg.text.is_empty() {
                msg.text = "[ no response ]".into();
            }
        }
    }
}

// ── ollama streaming call ─────────────────────────────────────────────────────
// runs in a separate tokio task, sends Token chunks through the channel
async fn stream_ollama(prompt: String, tx: mpsc::UnboundedSender<Token>) {
    use futures_util::StreamExt;

    let body = serde_json::json!({
        "model": MODEL,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": true
    });

    let client = reqwest::Client::new();
    let res = match client.post(OLLAMA).json(&body).send().await {
        Ok(r)  => r,
        Err(e) => { let _ = tx.send(Token::Err(e.to_string())); return; }
    };

    let mut stream = res.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let bytes: bytes::Bytes = match chunk {
            Ok(c)  => c,
            Err(e) => { let _ = tx.send(Token::Err(e.to_string())); return; }
        };

        let text = String::from_utf8_lossy(&bytes);
        for line in text.lines() {
            if line.is_empty() { continue; }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(content) = v["message"]["content"].as_str() {
                    if !content.is_empty() {
                        let _ = tx.send(Token::Delta(content.to_string()));
                    }
                }
                if v["done"].as_bool().unwrap_or(false) {
                    let _ = tx.send(Token::Done);
                    return;
                }
            }
        }
    }

    let _ = tx.send(Token::Done);
}

// ── entry point ───────────────────────────────────────────────────────────────
#[tokio::main]
async fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let mut app = App::new();

    // channel for streaming tokens from ollama task → main loop
    let (tx, mut rx) = mpsc::unbounded_channel::<Token>();

    loop {
        // ── drain any pending tokens from the stream ──────────────────────
        while let Ok(token) = rx.try_recv() {
            match token {
                Token::Delta(s) => { app.append_delta(&s); }
                Token::Done     => { app.finish_stream(); }
                Token::Err(e)   => {
                    app.append_delta(&format!("[ error: {e} ]"));
                    app.finish_stream();
                }
            }
        }

        app.tick = app.tick.wrapping_add(1);

        // ── draw ──────────────────────────────────────────────────────────
        terminal.draw(|f| {
            let area = f.area();
            f.render_widget(Block::default().style(Style::default().bg(BG)), area);

            let root = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(area);

            // header — animated dots while thinking
            let thinking = if app.streaming {
                let dots = match (app.tick / 20) % 3 {
                    0 => "·",
                    1 => "· ·",
                    _ => "· · ·",
                };
                format!("  thinking {}", dots)
            } else {
                String::new()
            };
            f.render_widget(
                Paragraph::new(format!("  MOAI v0.1.0{}", thinking))
                    .style(Style::default().fg(MUTED).bg(BG)),
                root[0],
            );

            f.render_widget(Block::default().style(Style::default().bg(SEP)), root[1]);

            // chat
            let mut lines: Vec<Line> = vec![Line::from("")];
            for msg in &app.messages {
                match msg.speaker {
                    Speaker::Logo => {
                        for l in msg.text.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("  {l}"),
                                Style::default().fg(MOAI_FG),
                            )));
                        }
                        lines.push(Line::from(""));
                    }
                    Speaker::User => {
                        lines.push(Line::from(vec![
                            Span::styled("  you   ", Style::default().fg(DIM)),
                            Span::styled(msg.text.clone(), Style::default().fg(USER_FG)),
                        ]));
                        lines.push(Line::from(""));
                    }
                    Speaker::Moai => {
                        let display = if msg.text.is_empty() {
                            "▋".to_string()
                        } else if app.streaming {
                            format!("{}▋", msg.text)
                        } else {
                            msg.text.clone()
                        };
                        lines.push(Line::from(vec![
                            Span::styled("  moai  ", Style::default().fg(DIM)),
                            Span::styled(display, Style::default().fg(MOAI_FG)),
                        ]));
                        lines.push(Line::from(""));
                    }
                }
            }

            let h = root[2].height;
            let total = lines.len() as u16;

            // if pinned, always resolve to real bottom
            if app.pinned_bottom {
                app.scroll = total.saturating_sub(h);
            }

            f.render_widget(
                Paragraph::new(Text::from(lines))
                    .style(Style::default().bg(BG))
                    .wrap(Wrap { trim: false })
                    .scroll((app.scroll, 0)),
                root[2],
            );

            f.render_widget(Block::default().style(Style::default().bg(SEP)), root[3]);

            f.render_widget(
                Paragraph::new("  ^C^C quit  ·  ↑↓ scroll")
                    .style(Style::default().fg(MUTED).bg(BG)),
                root[4],
            );

            f.render_widget(Block::default().style(Style::default().bg(SEP)), root[5]);

            f.render_widget(
                Paragraph::new(format!("  ❯ {}", app.input))
                    .style(Style::default().fg(USER_FG).bg(BG)),
                root[6],
            );
        })?;

        // ── events ────────────────────────────────────────────────────────
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                        if app.cc { break; } else { app.cc = true; }
                    }
                    KeyCode::Enter => {
                        if let Some(prompt) = app.submit() {
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                stream_ollama(prompt, tx2).await;
                            });
                        }
                    }
                    KeyCode::Char(c)   => { app.input.push(c); app.cc = false; }
                    KeyCode::Backspace => { app.input.pop(); app.cc = false; }
                    KeyCode::Up => {
                        app.pinned_bottom = false; // user scrolled up, stop following
                        app.scroll = app.scroll.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        app.scroll = app.scroll.saturating_add(1);
                        // re-pin if user scrolled back to bottom
                        // (actual clamping happens in draw, this is best-effort)
                    }
                    _                  => { app.cc = false; }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
