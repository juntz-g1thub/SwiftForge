use ratatui::style::{Style, Stylize};

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

#[derive(Debug, Clone, PartialEq)]
pub enum StreamingState {
    Idle,
    Streaming,
    Completed,
    Error(String),
}

impl StreamingState {
    pub fn is_active(&self) -> bool {
        matches!(self, StreamingState::Streaming)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, StreamingState::Completed | StreamingState::Error(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Reasoning,
    ToolCall,
}

#[derive(Debug, Clone)]
pub struct StreamingBlock {
    pub block_type: BlockType,
    pub title: String,
    pub content: String,
    pub status: StreamingState,
    width: usize,
}

impl StreamingBlock {
    pub fn new(block_type: BlockType, title: &str, width: usize) -> Self {
        Self {
            block_type,
            title: title.to_string(),
            content: String::new(),
            status: StreamingState::Streaming,
            width,
        }
    }

    pub fn append(&mut self, text: &str) {
        self.content.push_str(text);
    }

    pub fn set_completed(&mut self) {
        self.status = StreamingState::Completed;
    }

    pub fn render(self) -> Vec<ratatui::text::Line<'static>> {
        use ratatui::text::{Line, Span};

        let suffix = match self.status {
            StreamingState::Streaming => "...",
            StreamingState::Completed => " ✓",
            StreamingState::Error(_) => " ✗",
            StreamingState::Idle => "",
        };

        let inner_width = self.width.saturating_sub(2);
        let title_str = format!("{}{}", self.title, suffix);
        let top_dash_count = if self.width > title_str.len() + 7 {
            self.width.saturating_sub(title_str.len() + 7)
        } else {
            0
        };
        let bottom_dash_count = inner_width;

        let top = format!("┌─── {} {}┐", title_str, "─".repeat(top_dash_count));
        let bottom = format!("└{}┘", "─".repeat(bottom_dash_count));

        let style = match self.block_type {
            BlockType::Reasoning => Style::new().magenta(),
            BlockType::ToolCall => Style::new().cyan(),
        };

        let mut lines = Vec::new();
        lines.push(Line::from(vec![Span::styled(top, style)]));

        let content_owned = self.content.clone();
        let content_lines: Vec<String> = content_owned.lines().map(|s| s.to_string()).collect();
        for r_line in content_lines {
            let display_line = if r_line.len() > inner_width {
                format!("{}...│", &r_line[..inner_width.saturating_sub(5)])
            } else {
                let pad = " ".repeat(inner_width.saturating_sub(r_line.len()));
                format!("{}{} │", r_line, pad)
            };
            lines.push(Line::from(vec![Span::styled(
                format!("│{}", display_line),
                style,
            )]));
        }

        lines.push(Line::from(vec![Span::styled(bottom, style)]));
        lines
    }
}

#[derive(Debug, Clone)]
pub struct ToolCallBlock {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct MessageBlock {
    pub role: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCallBlock>,
    pub content: String,
    pub status: StreamingState,
}

impl MessageBlock {
    pub fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            reasoning: None,
            tool_calls: Vec::new(),
            content: content.to_string(),
            status: StreamingState::Completed,
        }
    }

    pub fn with_structured(
        role: &str,
        content: &str,
        reasoning: Option<String>,
        tool_calls: Vec<ToolCallBlock>,
    ) -> Self {
        Self {
            role: role.to_string(),
            reasoning,
            tool_calls,
            content: content.to_string(),
            status: StreamingState::Completed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatViewState {
    pub messages: Vec<MessageBlock>,
    pub input: String,
    pub cursor_pos: usize,
    pub scroll_offset: usize,
    pub content_height: usize,
    pub scrollbar_state: ratatui::widgets::ScrollbarState,
    pub streaming_state: StreamingState,
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
            streaming_state: StreamingState::Idle,
            current_provider: provider.to_string(),
            current_model: model.to_string(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(MessageBlock::new(role, content));
    }

    pub fn add_structured_message(
        &mut self,
        role: &str,
        content: &str,
        reasoning: Option<String>,
        tool_calls: Vec<ToolCallBlock>,
    ) {
        self.messages.push(MessageBlock::with_structured(
            role, content, reasoning, tool_calls,
        ));
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
