use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use crate::tui::state::{Action, AppContext, ChatViewState, StreamingState, UIState};
use crate::tui::views::View;
use ratatui::style::Style;

pub struct ChatView {
    pub state: ChatViewState,
    scrollbar_state: ScrollbarState,
}

impl ChatView {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            state: ChatViewState::new(provider, model),
            scrollbar_state: ScrollbarState::new(0),
        }
    }

    fn display_width(c: char) -> usize {
        match c {
            c if c.is_ascii() => 1,
            c if c == '\t' => 4,
            c if c >= '\u{3000}' && c <= '\u{303F}' => 2,
            c if c >= '\u{3040}' && c <= '\u{309F}' => 2,
            c if c >= '\u{30A0}' && c <= '\u{30FF}' => 2,
            _ if c >= '\u{4E00}' && c <= '\u{9FFF}' => 2,
            _ => 1,
        }
    }

    fn text_width(text: &str) -> usize {
        text.chars().map(Self::display_width).sum()
    }

    fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
        let mut char_count = 0;
        for (byte_idx, _c) in s.char_indices() {
            if char_count >= char_idx {
                return byte_idx;
            }
            char_count += 1;
        }
        s.len()
    }

    fn format_prefix(role: &str, model: &str) -> String {
        match role {
            "user" => format!("[{}]: ", role),
            _ => format!("[{} {}]: ", role, model),
        }
    }

    fn remove_char_before_cursor(input: &mut String, cursor_pos: &mut usize) {
        if *cursor_pos == 0 || input.is_empty() {
            return;
        }
        let target_char_pos = *cursor_pos - 1;
        let byte_idx = Self::char_to_byte_idx(input, target_char_pos);
        input.remove(byte_idx);
        *cursor_pos = target_char_pos;
    }

    fn render_reasoning_block(
        &self,
        f: &mut Frame,
        area: Rect,
        reasoning: &str,
        is_streaming: bool,
    ) {
        let title = if is_streaming {
            "🌙 DeepSeek Reasoning..."
        } else {
            "🌙 DeepSeek Reasoning ✓"
        };

        let border_style = Style::new().magenta();

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::new().on_green().black());

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        let lines: Vec<Line> = reasoning.lines().map(Line::from).collect();

        let paragraph = Paragraph::new(Text::from(lines));

        f.render_widget(paragraph, inner_area);
    }

    fn render_tool_call_block(&self, f: &mut Frame, area: Rect, name: &str, arguments: &str) {
        let title = format!("🔧 Tool Call: {}", name);

        let border_style = Style::new().cyan();

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::new().on_blue().black());

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        let content = format!("{}\n\nResult: ...", arguments);
        let lines: Vec<Line> = content.lines().map(Line::from).collect();

        let paragraph = Paragraph::new(Text::from(lines));
        f.render_widget(paragraph, inner_area);
    }

    fn render_messages(&mut self, f: &mut Frame, area: Rect, ui_state: &UIState) {
        let mut lines: Vec<Line> = Vec::new();

        for msg in &self.state.messages {
            let label_style = match msg.role.as_str() {
                "user" => Style::new().green().bold(),
                "assistant" => Style::new().cyan().bold(),
                "system" => Style::new().yellow().bold(),
                "error" => Style::new().red().bold(),
                _ => Style::new().cyan().bold(),
            };
            let label = Self::format_prefix(&msg.role, &self.state.current_model);
            lines.push(Line::from(Span::styled(label, label_style)));

            // Reasoning block — text-embedded with box-drawing borders
            if let Some(ref reasoning) = msg.reasoning {
                let width = area.width as usize;
                let top = format!("┌─── Reasoning {}", "─".repeat(width.saturating_sub(16)));
                let bottom = format!("└{}┘", "─".repeat(width.saturating_sub(2)));
                lines.push(Line::from(Span::styled(top, Style::new().magenta())));
                for r_line in reasoning.lines() {
                    let inner = if r_line.len() + 4 > width {
                        format!(" {}...│", &r_line[..width.saturating_sub(7)])
                    } else {
                        let pad = " ".repeat(width.saturating_sub(r_line.len() + 3));
                        format!(" {}{} │", r_line, pad)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("│{}", inner),
                        Style::new().magenta(),
                    )));
                }
                lines.push(Line::from(Span::styled(bottom, Style::new().magenta())));
            }

            // Content output
            lines.push(Line::from(Span::raw(msg.content.clone())));
        }

        if self.state.streaming_state.is_active() {
            if let Ok(streaming) = ui_state.streaming_text.lock() {
                if let Some(ref text) = *streaming {
                    lines.push(Line::from(Span::styled(
                        Self::format_prefix("assistant", &self.state.current_model),
                        Style::new().cyan().bold(),
                    )));
                    lines.push(Line::from(Span::raw(text.clone())));
                    lines.push(Line::from(Span::styled("▌", Style::new().slow_blink())));
                }
            }
        }

        self.state.content_height = lines.len();
        let visible_height = area.height as usize;

        let max_scroll = if self.state.content_height > visible_height {
            self.state.content_height - visible_height
        } else {
            0
        };
        if self.state.scroll_offset > max_scroll {
            self.state.scroll_offset = max_scroll;
        }
        self.scrollbar_state = ScrollbarState::new(max_scroll).position(self.state.scroll_offset);

        let scrollable_lines: Vec<Line> = if self.state.scroll_offset >= lines.len() {
            lines.clone()
        } else {
            lines[self.state.scroll_offset..].to_vec()
        };

        let paragraph = Paragraph::new(Text::from(scrollable_lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Chat (Ctrl+S: Settings)"),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);

        let scrollbar_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 0,
        });
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            scrollbar_area,
            &mut self.scrollbar_state,
        );
    }

    fn render_input(&self, f: &mut Frame, area: Rect) {
        let display_input = if self.state.input.is_empty() {
            String::from("> ")
        } else {
            let byte_pos = Self::char_to_byte_idx(
                &self.state.input,
                self.state.cursor_pos.min(self.state.input.chars().count()),
            );
            let before = &self.state.input[..byte_pos];
            let after = &self.state.input[byte_pos..];
            format!("> {}{}▌", before, after)
        };

        let input_para = Paragraph::new(display_input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input_para, area);

        let char_offset: u16 = self
            .state
            .input
            .chars()
            .take(self.state.cursor_pos)
            .map(Self::display_width)
            .sum::<usize>() as u16;
        let cursor_x = area.x + 1 + 2 + char_offset;
        let cursor_y = area.y + 1;
        f.set_cursor(cursor_x, cursor_y);
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let status_text = format!(
            "[{}] - Press Ctrl+Q to quit, Ctrl+S for settings",
            self.state.current_provider
        );
        let status_para = Paragraph::new(status_text.as_str());
        f.render_widget(status_para, area);
    }
}

impl View for ChatView {
    fn render(&mut self, f: &mut Frame, area: Rect, _ctx: &AppContext, ui_state: &UIState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area);

        self.render_messages(f, chunks[0], ui_state);
        self.render_input(f, chunks[1]);
        self.render_status(f, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &AppContext) -> Option<Action> {
        if self.state.streaming_state.is_active() {
            match key.code {
                KeyCode::Up => {
                    if self.state.scroll_offset > 0 {
                        self.state.scroll_offset -= 1;
                    }
                    Some(Action::ScrollUp)
                }
                KeyCode::Down => {
                    let max_scroll = self.state.content_height.saturating_sub(1);
                    if self.state.scroll_offset < max_scroll {
                        self.state.scroll_offset += 1;
                    }
                    Some(Action::ScrollDown)
                }
                KeyCode::Esc => {
                    self.state.streaming_state = StreamingState::Idle;
                    Some(Action::CancelStreaming)
                }
                _ => None,
            }
        } else {
            match (key.code, key.modifiers) {
                (KeyCode::Char(c), KeyModifiers::CONTROL) => match c {
                    'q' | 'Q' => Some(Action::Quit),
                    's' | 'S' => Some(Action::SwitchView(crate::tui::ViewState::Config(
                        crate::tui::ConfigContext::default(),
                    ))),
                    _ => None,
                },
                (KeyCode::Char(c), _) => {
                    self.state.input.push(c);
                    self.state.cursor_pos += 1;
                    None
                }
                (KeyCode::Backspace, _) => {
                    Self::remove_char_before_cursor(
                        &mut self.state.input,
                        &mut self.state.cursor_pos,
                    );
                    None
                }
                (KeyCode::Delete, _) => {
                    let char_count = self.state.input.chars().count();
                    if self.state.cursor_pos < char_count && !self.state.input.is_empty() {
                        let byte_idx =
                            Self::char_to_byte_idx(&self.state.input, self.state.cursor_pos);
                        self.state.input.remove(byte_idx);
                    }
                    None
                }
                (KeyCode::Left, _) => {
                    if self.state.cursor_pos > 0 {
                        self.state.cursor_pos -= 1;
                    }
                    None
                }
                (KeyCode::Right, _) => {
                    if self.state.cursor_pos < self.state.input.chars().count() {
                        self.state.cursor_pos += 1;
                    }
                    None
                }
                (KeyCode::Home, _) => {
                    self.state.cursor_pos = 0;
                    None
                }
                (KeyCode::End, _) => {
                    self.state.cursor_pos = self.state.input.chars().count();
                    None
                }
                (KeyCode::Up, _) => {
                    if self.state.scroll_offset > 0 {
                        self.state.scroll_offset -= 1;
                    }
                    Some(Action::ScrollUp)
                }
                (KeyCode::Down, _) => {
                    let max_scroll = self.state.content_height.saturating_sub(1);
                    if self.state.scroll_offset < max_scroll {
                        self.state.scroll_offset += 1;
                    }
                    Some(Action::ScrollDown)
                }
                (KeyCode::Enter, _) => {
                    if !self.state.input.trim().is_empty() {
                        let msg = self.state.input.trim().to_string();
                        self.state.input.clear();
                        self.state.cursor_pos = 0;
                        self.state.streaming_state = StreamingState::Streaming;
                        Some(Action::SendMessage(msg))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }
}
