use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct MessageList;

impl MessageList {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, f: &mut Frame, area: Rect, messages: &[String]) {
        let content = messages.join("\n");
        let paragraph = Paragraph::new(content).block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Messages"),
        );
        f.render_widget(paragraph, area);
    }
}

pub struct InputArea;

impl InputArea {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, f: &mut Frame, area: Rect, input: &str) {
        let paragraph = Paragraph::new(input.to_string()).block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Input"),
        );
        f.render_widget(paragraph, area);
    }
}

pub struct StatusBar;

impl StatusBar {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let text = "Rust Agent Platform | Ctrl+C to quit";
        let paragraph = Paragraph::new(text);
        f.render_widget(paragraph, area);
    }
}
