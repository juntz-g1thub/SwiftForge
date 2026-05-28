#[derive(Debug, Clone)]
pub struct ChatContext {
    pub current_provider: String,
    pub current_model: String,
}

impl ChatContext {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            current_provider: provider.to_string(),
            current_model: model.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConfigContext {
    pub editing_provider: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ViewState {
    Chat(ChatContext),
    Config(ConfigContext),
}

impl ViewState {
    pub fn as_chat(&self) -> Option<&ChatContext> {
        match self {
            ViewState::Chat(ctx) => Some(ctx),
            _ => None,
        }
    }

    pub fn as_config(&self) -> Option<&ConfigContext> {
        match self {
            ViewState::Config(ctx) => Some(ctx),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatViewState {
    pub messages: Vec<(String, String)>,
    pub input: String,
    pub cursor_pos: usize,
    pub scroll_offset: usize,
    pub content_height: usize,
    pub scrollbar_state: ratatui::widgets::ScrollbarState,
    pub debug_scrollbar_state: ratatui::widgets::ScrollbarState,
    pub debug_scroll_offset: usize,
    pub debug_content_height: usize,
    pub is_streaming: bool,
    pub current_provider: String,
    pub current_model: String,
}

impl ChatViewState {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            content_height: 0,
            scrollbar_state: ratatui::widgets::ScrollbarState::new(0),
            debug_scrollbar_state: ratatui::widgets::ScrollbarState::new(0),
            debug_scroll_offset: 0,
            debug_content_height: 0,
            is_streaming: false,
            current_provider: provider.to_string(),
            current_model: model.to_string(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push((role.to_string(), content.to_string()));
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConfigViewState {
    pub editing_provider: Option<String>,
}

impl ConfigViewState {
    pub fn new() -> Self {
        Self::default()
    }
}
