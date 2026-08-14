use crate::browser::Browser;
use crate::extract::{Document, Kind};
use crate::render;
use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

enum Prompt {
    Url,
    Js,
    Field(u32),
}

impl Prompt {
    fn label(&self) -> &'static str {
        match self {
            Prompt::Url => "url> ",
            Prompt::Js => "js> ",
            Prompt::Field(_) => "type> ",
        }
    }
}

pub struct App {
    browser: Browser,
    doc: Document,
    scroll: u16,
    focus: Option<u32>,
    digits: String,
    prompt: Option<(Prompt, String)>,
    console: String,
    status: String,
    quit: bool,
}

impl App {
    pub async fn new(browser: Browser, url: &str) -> Result<Self> {
        browser.goto(url).await?;
        let doc = browser.extract().await?;
        Ok(Self {
            browser,
            doc,
            scroll: 0,
            focus: None,
            digits: String::new(),
            prompt: None,
            console: String::new(),
            status: String::new(),
            quit: false,
        })
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<Browser> {
        let mut events = EventStream::new();
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            if let Some(Ok(event)) = events.next().await {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')
                    {
                        break;
                    }
                    if self.prompt.is_some() {
                        self.prompt_key(key.code).await?;
                    } else {
                        self.normal_key(key.code).await?;
                    }
                }
            }
        }
        Ok(self.browser)
    }

    async fn normal_key(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageDown | KeyCode::Char(' ') => self.scroll = self.scroll.saturating_add(20),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(20),
            KeyCode::Char('g') => self.scroll = 0,
            KeyCode::Char('G') => self.scroll = self.line_count().saturating_sub(1),
            KeyCode::Tab => self.cycle_focus(1),
            KeyCode::BackTab => self.cycle_focus(-1),
            KeyCode::Char(c) if c.is_ascii_digit() => self.digits.push(c),
            KeyCode::Esc => self.digits.clear(),
            KeyCode::Char(':') => self.prompt = Some((Prompt::Url, String::new())),
            KeyCode::Char('`') => self.prompt = Some((Prompt::Js, String::new())),
            KeyCode::Char('r') => self.reload().await?,
            KeyCode::Char('i') => {
                if let Some(id) = self.focus {
                    self.prompt = Some((Prompt::Field(id), String::new()));
                }
            }
            KeyCode::Enter => self.activate().await?,
            _ => {}
        }
        Ok(())
    }

    async fn prompt_key(&mut self, code: KeyCode) -> Result<()> {
        let Some((kind, buffer)) = self.prompt.as_mut() else { return Ok(()) };
        match code {
            KeyCode::Esc => self.prompt = None,
            KeyCode::Backspace => {
                buffer.pop();
            }
            KeyCode::Char(c) => buffer.push(c),
            KeyCode::Enter => {
                let text = buffer.clone();
                let kind = match kind {
                    Prompt::Url => Prompt::Url,
                    Prompt::Js => Prompt::Js,
                    Prompt::Field(id) => Prompt::Field(*id),
                };
                self.prompt = None;
                self.submit_prompt(kind, text).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn submit_prompt(&mut self, kind: Prompt, text: String) -> Result<()> {
        match kind {
            Prompt::Url => {
                let url = if text.contains("://") { text } else { format!("https://{text}") };
                match self.browser.goto(&url).await {
                    Ok(()) => self.reload().await?,
                    Err(e) => self.status = format!("goto failed: {e}"),
                }
            }
            Prompt::Js => match self.browser.eval(&text).await {
                Ok(value) => {
                    self.console = serde_json::to_string_pretty(&value)?;
                }
                Err(e) => self.console = format!("error: {e}"),
            },
            Prompt::Field(id) => {
                self.browser.type_into(id, &text).await?;
                self.browser.submit(id).await?;
                self.reload().await?;
            }
        }
        Ok(())
    }

    async fn activate(&mut self) -> Result<()> {
        if !self.digits.is_empty() {
            let wanted: u32 = self.digits.parse().unwrap_or(0);
            self.digits.clear();
            if self.doc.addressable().contains(&wanted) {
                self.focus = Some(wanted);
                self.status = format!("focus [{wanted}]");
            } else {
                self.status = format!("no element [{wanted}]");
            }
            return Ok(());
        }
        let Some(id) = self.focus else { return Ok(()) };
        match self.focused_kind() {
            Some(Kind::Input) => self.prompt = Some((Prompt::Field(id), String::new())),
            _ => {
                self.browser.click(id).await?;
                self.reload().await?;
            }
        }
        Ok(())
    }

    async fn reload(&mut self) -> Result<()> {
        self.doc = self.browser.extract().await?;
        self.scroll = 0;
        // Ids are re-issued on every extraction, so a stale focus would point elsewhere.
        self.focus = None;
        self.status = format!("{} blocks", self.doc.blocks.len());
        Ok(())
    }

    fn focused_kind(&self) -> Option<Kind> {
        let id = self.focus?;
        self.doc.blocks.iter().find(|b| b.id == Some(id)).map(|b| b.kind)
    }

    fn cycle_focus(&mut self, step: i32) {
        let ids = self.doc.addressable();
        if ids.is_empty() {
            return;
        }
        let current = self.focus.and_then(|id| ids.iter().position(|&i| i == id));
        let next = match (current, step) {
            (None, 1) => 0,
            (None, _) => ids.len() - 1,
            (Some(i), 1) => (i + 1) % ids.len(),
            (Some(i), _) => (i + ids.len() - 1) % ids.len(),
        };
        self.focus = Some(ids[next]);
    }

    fn line_count(&self) -> u16 {
        render::lines(&self.doc, self.focus).len() as u16
    }

    fn draw(&mut self, frame: &mut Frame) {
        let console_height = if self.console.is_empty() { 0 } else { 8 };
        let [content, console, status] = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(console_height),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        let lines = render::lines(&self.doc, self.focus);
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }).scroll((self.scroll, 0)),
            content,
        );

        if console_height > 0 {
            frame.render_widget(
                Paragraph::new(self.console.clone())
                    .block(Block::default().borders(Borders::TOP).title(" js "))
                    .wrap(Wrap { trim: false }),
                console,
            );
        }

        let bar = match &self.prompt {
            Some((kind, buffer)) => format!("{}{}_", kind.label(), buffer),
            None => {
                let digits =
                    if self.digits.is_empty() { String::new() } else { format!(" #{}", self.digits) };
                // A focused link shows where it goes, which is the one thing the
                // rendered text cannot tell you.
                let target = self
                    .focus
                    .and_then(|id| self.doc.blocks.iter().find(|b| b.id == Some(id)))
                    .map(|b| match &b.href {
                        Some(href) => format!(" [{}] -> {href}", b.id.unwrap_or(0)),
                        None => format!(" [{}]", b.id.unwrap_or(0)),
                    })
                    .unwrap_or_else(|| format!(" {}", self.doc.url));
                format!("{}{target}{digits}  {}", self.doc.title, self.status)
            }
        };
        frame.render_widget(
            Paragraph::new(bar).style(
                Style::default().fg(Color::Black).bg(Color::Gray).add_modifier(Modifier::BOLD),
            ),
            status,
        );
    }
}
