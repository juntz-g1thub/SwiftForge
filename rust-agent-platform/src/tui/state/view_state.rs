use super::action::ViewStateKind;

#[derive(Debug, Clone)]
pub enum ViewState {
    Chat(ChatViewState),
    Config(ConfigViewState),
    Debug(DebugViewState),
}

impl ViewState {
    pub fn kind(&self) -> ViewStateKind {
        match self {
            ViewState::Chat(_) => ViewStateKind::Chat,
            ViewState::Config(_) => ViewStateKind::Config,
            ViewState::Debug(_) => ViewStateKind::Debug,
        }
    }

    pub fn into_chat(self) -> Option<ChatViewState> {
        if let ViewState::Chat(state) = self {
            Some(state)
        } else {
            None
        }
    }

    pub fn into_config(self) -> Option<ConfigViewState> {
        if let ViewState::Config(state) = self {
            Some(state)
        } else {
            None
        }
    }

    pub fn into_debug(self) -> Option<DebugViewState> {
        if let ViewState::Debug(state) = self {
            Some(state)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatViewState {
    pub messages: Vec<(String, String)>,
    pub input: String,
    pub cursor_pos: usize,
    pub is_streaming: bool,
    pub scroll_offset: usize,
    pub content_height: usize,
    pub streaming_text: Option<String>,
    pub current_provider: String,
    pub current_model: String,
    pub debug_scroll_offset: usize,
    pub debug_content_height: usize,
}

impl ChatViewState {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            is_streaming: false,
            scroll_offset: 0,
            content_height: 0,
            streaming_text: None,
            current_provider: provider.to_string(),
            current_model: model.to_string(),
            debug_scroll_offset: 0,
            debug_content_height: 0,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push((role.to_string(), content.to_string()));
    }
}

#[derive(Debug, Clone)]
pub enum ConfigViewState {
    SelectProvider,
    Editing(ProviderEditStage),
    FetchingModels { error: Option<String> },
    SelectModel(Vec<String>),
}

#[derive(Debug, Clone)]
pub enum ProviderEditStage {
    SelectProvider,
    ApiKey,
    Model,
    BaseUrl,
    CustomName,
    CustomUrl,
}

impl ConfigViewState {
    pub fn new() -> Self {
        ConfigViewState::SelectProvider
    }
}

#[derive(Debug, Clone)]
pub struct DebugViewState {
    pub messages: Vec<String>,
    pub scroll_offset: usize,
    pub content_height: usize,
}

impl DebugViewState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            content_height: 0,
        }
    }
}
