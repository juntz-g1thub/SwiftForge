use super::action::ViewStateKind;

#[derive(Debug, Clone)]
pub enum MessageStatus {
    Streaming,
    Completed,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ToolCallBlock {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct ToolResultBlock {
    pub tool_name: String,
    pub output: String,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct MessageBlock {
    pub role: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCallBlock>,
    pub tool_results: Vec<ToolResultBlock>,
    pub content: String,
    pub status: MessageStatus,
}

impl MessageBlock {
    pub fn new_user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            reasoning: None,
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            content,
            status: MessageStatus::Completed,
        }
    }

    pub fn new_assistant() -> Self {
        Self {
            role: "assistant".to_string(),
            reasoning: None,
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            content: String::new(),
            status: MessageStatus::Streaming,
        }
    }

    pub fn with_reasoning(mut self, reasoning: String) -> Self {
        self.reasoning = Some(reasoning);
        self
    }

    pub fn with_tool_call(mut self, name: String, arguments: String) -> Self {
        self.tool_calls.push(ToolCallBlock { name, arguments });
        self
    }

    pub fn with_tool_result(mut self, tool_name: String, output: String, success: bool) -> Self {
        self.tool_results.push(ToolResultBlock {
            tool_name,
            output,
            success,
        });
        self
    }

    pub fn with_content(mut self, content: String) -> Self {
        self.content = content;
        self
    }

    pub fn set_completed(mut self) -> Self {
        self.status = MessageStatus::Completed;
        self
    }

    pub fn set_error(mut self, error: String) -> Self {
        self.status = MessageStatus::Error(error);
        self
    }
}

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
    pub messages: Vec<MessageBlock>,
    pub input: String,
    pub cursor_pos: usize,
    pub is_streaming: bool,
    pub scroll_offset: usize,
    pub content_height: usize,
    pub streaming_text: Option<String>,
    pub current_provider: String,
    pub current_model: String,
    pub reasoning_collapsed: bool,
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
            reasoning_collapsed: false,
            debug_scroll_offset: 0,
            debug_content_height: 0,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        let mut msg = MessageBlock::new_user(content.to_string());
        msg.role = role.to_string();
        self.messages.push(msg);
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
