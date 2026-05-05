use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use std::io::Stdout;

pub struct App {
    messages: Vec<String>,
    input: String,
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            should_quit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key)?;
                if self.should_quit {
                    break;
                }
            }
        }
        Ok(())
    }

    fn render(&self, f: &mut Frame) {
        use ratatui::layout::{Constraint, Direction, Layout, Rect};
        use ratatui::widgets::{Block, Borders, Paragraph};

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(f.size());

        let messages = Paragraph::new(self.messages.join("\n"))
            .block(Block::default().borders(Borders::ALL).title("Messages"));
        f.render_widget(messages, chunks[0]);

        let input = Paragraph::new(self.input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input, chunks[1]);

        let status = Paragraph::new("Rust Agent Platform | Ctrl+C to quit");
        f.render_widget(status, chunks[2]);
    }

    fn handle_key_event(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                    self.should_quit = true;
                } else if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.input.push(c);
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Enter => {
                self.messages.push(self.input.clone());
                self.input.clear();
            }
            _ => {}
        }
        Ok(())
    }
}
