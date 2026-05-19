use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::state::{Action, AppContext, ConfigViewState, ProviderEditStage, UIState};
use crate::tui::views::View;

pub struct ConfigView {
    pub state: ConfigViewState,
    input_buffer: String,
    cursor_pos: usize,
}

impl ConfigView {
    pub fn new() -> Self {
        Self {
            state: ConfigViewState::new(),
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
            Paragraph::new("[F] Fetch Models | [ESC] Back to Chat"),
            chunks[2],
        );
    }

    fn render_input_field(&self, f: &mut Frame, area: Rect, title: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);

        let input_para = Paragraph::new(self.input_buffer.as_str())
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(input_para, chunks[0]);

        let hint_para = Paragraph::new("[ENTER] Save | [ESC] Cancel");
        f.render_widget(hint_para, chunks[1]);
    }

    fn render_fetching(&self, f: &mut Frame, area: Rect, error: Option<&str>) {
        let message = if let Some(err) = error {
            format!("Error fetching models: {}", err)
        } else {
            String::from("Fetching models...")
        };

        let para = Paragraph::new(message)
            .block(Block::default().borders(Borders::ALL).title("Fetch Models"));
        f.render_widget(para, area);

        let hint = Paragraph::new("[ESC] Cancel");
        f.render_widget(hint, area);
    }

    fn render_model_list(&self, f: &mut Frame, area: Rect, models: &[String]) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        f.render_widget(
            Paragraph::new("Select Model")
                .block(Block::default().borders(Borders::ALL).title("Models")),
            chunks[0],
        );

        let list = Paragraph::new(Text::from(models.join("\n")))
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(list, chunks[1]);

        f.render_widget(Paragraph::new("[ENTER] Select | [ESC] Cancel"), chunks[2]);
    }
}

impl View for ConfigView {
    fn render(&mut self, f: &mut Frame, area: Rect, ctx: &AppContext, _ui_state: &UIState) {
        match &self.state {
            ConfigViewState::SelectProvider => {
                let provider = ctx.config.lock().unwrap().get_provider().to_string();
                self.render_provider_list(f, area, &provider);
            }
            ConfigViewState::Editing(stage) => {
                let title = match stage {
                    ProviderEditStage::ApiKey => "API Key",
                    ProviderEditStage::Model => "Model",
                    ProviderEditStage::BaseUrl => "Base URL",
                    ProviderEditStage::CustomName => "Custom Provider Name",
                    ProviderEditStage::CustomUrl => "Custom Base URL",
                    ProviderEditStage::SelectProvider => "Select Provider",
                };
                self.render_input_field(f, area, title);
            }
            ConfigViewState::FetchingModels { error } => {
                self.render_fetching(f, area, error.as_deref());
            }
            ConfigViewState::SelectModel(models) => {
                self.render_model_list(f, area, models);
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent, ctx: &AppContext) -> Option<Action> {
        match &self.state {
            ConfigViewState::SelectProvider => match key.code {
                KeyCode::Char('1') => Some(Action::SelectProvider("openai".to_string())),
                KeyCode::Char('2') => Some(Action::SelectProvider("anthropic".to_string())),
                KeyCode::Char('3') => Some(Action::SelectProvider("ollama".to_string())),
                KeyCode::Char('4') => Some(Action::SelectProvider("deepseek".to_string())),
                KeyCode::Char('5') => Some(Action::SelectProvider("minimax".to_string())),
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.state = ConfigViewState::Editing(ProviderEditStage::SelectProvider);
                    None
                }
                KeyCode::Char('f') | KeyCode::Char('F') => Some(Action::FetchModels),
                KeyCode::Esc => Some(Action::SwitchView(crate::tui::ViewState::Chat(
                    crate::tui::ChatViewState::new(
                        &ctx.config.lock().unwrap().get_provider(),
                        &ctx.config
                            .lock()
                            .unwrap()
                            .get_model(ctx.config.lock().unwrap().get_provider()),
                    ),
                ))),
                _ => None,
            },
            ConfigViewState::Editing(stage) => match key.code {
                KeyCode::Enter => {
                    let action = match stage {
                        ProviderEditStage::ApiKey => Action::SaveApiKey(self.input_buffer.clone()),
                        ProviderEditStage::Model => Action::SaveModel(self.input_buffer.clone()),
                        ProviderEditStage::BaseUrl => {
                            Action::SaveBaseUrl(self.input_buffer.clone())
                        }
                        _ => return None,
                    };
                    self.input_buffer.clear();
                    self.cursor_pos = 0;
                    Some(action)
                }
                KeyCode::Esc => {
                    self.input_buffer.clear();
                    self.cursor_pos = 0;
                    self.state = ConfigViewState::SelectProvider;
                    None
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                    self.cursor_pos += 1;
                    None
                }
                KeyCode::Backspace => {
                    if self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                        self.input_buffer.pop();
                    }
                    None
                }
                _ => None,
            },
            ConfigViewState::FetchingModels { .. } => {
                if let KeyCode::Esc = key.code {
                    self.state = ConfigViewState::SelectProvider;
                    None
                } else {
                    None
                }
            }
            ConfigViewState::SelectModel(_) => {
                if let KeyCode::Esc = key.code {
                    self.state = ConfigViewState::SelectProvider;
                    None
                } else {
                    None
                }
            }
        }
    }
}
