use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::state::{Action, AppContext, ConfigContext, UIState};
use crate::tui::views::View;

pub struct ConfigView {
    pub state: ConfigContext,
    input_buffer: String,
    cursor_pos: usize,
}

impl ConfigView {
    pub fn new() -> Self {
        Self {
            state: ConfigContext::default(),
            input_buffer: String::new(),
            cursor_pos: 0,
        }
    }

    fn render_provider_list(&self, f: &mut Frame, area: Rect, current_provider: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);

        f.render_widget(
            Paragraph::new(format!("Current: {}", current_provider))
                .block(Block::default().borders(Borders::ALL).title("Provider")),
            chunks[0],
        );

        let providers = vec![
            ("openai", "OpenAI"),
            ("anthropic", "Anthropic"),
            ("ollama", "Ollama"),
            ("deepseek", "DeepSeek"),
            ("minimax", "MiniMax"),
            ("custom", "Custom"),
        ];

        let mut items = Vec::new();
        for (id, name) in providers {
            let marker = if id == current_provider { ">" } else { " " };
            items.push(format!("{} [{}]", marker, name));
        }

        let list = Paragraph::new(Text::from(items.join("\n"))).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select Provider"),
        );

        f.render_widget(list, chunks[1]);

        f.render_widget(
            Paragraph::new("[1-6] Select | [ESC] Back to Chat"),
            chunks[2],
        );
    }
}

impl View for ConfigView {
    fn render(&mut self, f: &mut Frame, area: Rect, ctx: &AppContext, _ui_state: &UIState) {
        let provider = ctx.config.lock().unwrap().get_provider().to_string();
        self.render_provider_list(f, area, &provider);
    }

    fn handle_key(&mut self, key: KeyEvent, ctx: &AppContext) -> Option<Action> {
        match key.code {
            KeyCode::Char('1') => Some(Action::SelectProvider("openai".to_string())),
            KeyCode::Char('2') => Some(Action::SelectProvider("anthropic".to_string())),
            KeyCode::Char('3') => Some(Action::SelectProvider("ollama".to_string())),
            KeyCode::Char('4') => Some(Action::SelectProvider("deepseek".to_string())),
            KeyCode::Char('5') => Some(Action::SelectProvider("minimax".to_string())),
            KeyCode::Char('6') => Some(Action::SelectProvider("custom".to_string())),
            KeyCode::Esc => Some(Action::SwitchView(crate::tui::ViewState::Chat(
                crate::tui::ChatContext::new(
                    &ctx.config.lock().unwrap().get_provider(),
                    &ctx.config
                        .lock()
                        .unwrap()
                        .get_model(ctx.config.lock().unwrap().get_provider()),
                ),
            ))),
            _ => None,
        }
    }
}
