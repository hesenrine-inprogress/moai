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

enum Speaker { Logo, User, Moai }
struct Message { speaker: Speaker, text: String }

struct App {
    messages: Vec<Message>,
    input:    String,
    scroll:   u16,
    cc:       bool,
}

impl App {
    fn new() -> Self {
        Self {
            messages: vec![Message { speaker: Speaker::Logo, text: LOGO.to_string() }],
            input:    String::new(),
            scroll:   0,
            cc:       false,
        }
    }

    fn submit(&mut self) {
        let t = self.input.trim().to_string();
        if t.is_empty() { return; }
        self.messages.push(Message { speaker: Speaker::User, text: t });
        self.messages.push(Message { speaker: Speaker::Moai, text: "[ no agent connected ]".into() });
        self.input.clear();
        self.scroll = u16::MAX;
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let mut app = App::new();

    loop {
        terminal.draw(|f| {
            let area = f.area();
            f.render_widget(Block::default().style(Style::default().bg(BG)), area);

            let root = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),  // header
                    Constraint::Length(1),  // sep
                    Constraint::Min(0),     // chat
                    Constraint::Length(1),  // sep
                    Constraint::Length(1),  // footer
                    Constraint::Length(1),  // sep
                    Constraint::Length(1),  // input
                ])
                .split(area);

            // header
            f.render_widget(
                Paragraph::new("  MOAI v0.1.0")
                    .style(Style::default().fg(MUTED).bg(BG)),
                root[0],
            );

            // sep
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
                        lines.push(Line::from(vec![
                            Span::styled("  moai  ", Style::default().fg(DIM)),
                            Span::styled(msg.text.clone(), Style::default().fg(MOAI_FG)),
                        ]));
                        lines.push(Line::from(""));
                    }
                }
            }

            let h = root[2].height;
            let total = lines.len() as u16;
            if app.scroll == u16::MAX {
                app.scroll = total.saturating_sub(h);
            }

            f.render_widget(
                Paragraph::new(Text::from(lines))
                    .style(Style::default().bg(BG))
                    .wrap(Wrap { trim: false })
                    .scroll((app.scroll, 0)),
                root[2],
            );

            // sep
            f.render_widget(Block::default().style(Style::default().bg(SEP)), root[3]);

            // footer
            f.render_widget(
                Paragraph::new("  ^C^C quit  ·  ↑↓ scroll")
                    .style(Style::default().fg(MUTED).bg(BG)),
                root[4],
            );

            // sep
            f.render_widget(Block::default().style(Style::default().bg(SEP)), root[5]);

            // input
            f.render_widget(
                Paragraph::new(format!("  ❯ {}", app.input))
                    .style(Style::default().fg(USER_FG).bg(BG)),
                root[6],
            );
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                        if app.cc { break; } else { app.cc = true; }
                    }
                    KeyCode::Enter     => { app.submit(); }
                    KeyCode::Char(c)   => { app.input.push(c); app.cc = false; }
                    KeyCode::Backspace => { app.input.pop(); app.cc = false; }
                    KeyCode::Up        => { app.scroll = app.scroll.saturating_sub(1); }
                    KeyCode::Down      => { app.scroll = app.scroll.saturating_add(1); }
                    _                  => { app.cc = false; }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
