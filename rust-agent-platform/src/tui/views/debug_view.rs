use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::tui::state::{Action, AppContext, DebugViewState, UIState};
use crate::tui::views::View;

pub struct DebugView {
    pub state: DebugViewState,
    scrollbar_state: ScrollbarState,
}

impl DebugView {
    pub fn new() -> Self {
        Self {
            state: DebugViewState::new(),
            scrollbar_state: ScrollbarState::new(0),
        }
    }
}

impl View for DebugView {
    fn render(&mut self, f: &mut Frame, area: Rect, _ctx: &AppContext, ui_state: &UIState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        let messages: Vec<String> = if let Ok(guard) = ui_state.debug_messages.lock() {
            guard.clone()
        } else {
            Vec::new()
        };

        self.state.content_height = messages.len();
        let visible_height = chunks[0].height as usize;

        let max_scroll = if self.state.content_height > visible_height {
            self.state.content_height - visible_height
        } else {
            0
        };
        if self.state.scroll_offset > max_scroll {
            self.state.scroll_offset = max_scroll;
        }
        self.scrollbar_state = ScrollbarState::new(max_scroll).position(self.state.scroll_offset);

        let visible_lines: Vec<Line> = if self.state.scroll_offset >= messages.len() {
            messages.iter().map(|m| Line::from(m.clone())).collect()
        } else {
            messages[self.state.scroll_offset..]
                .iter()
                .map(|m| Line::from(m.clone()))
                .collect()
        };

        let paragraph = Paragraph::new(Text::from(visible_lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Debug Log (↑↓ scroll)"),
            )
            .style(Style::default().fg(Color::Red));

        f.render_widget(paragraph, chunks[0]);

        let scrollbar_area = chunks[0].inner(&Margin {
            vertical: 1,
            horizontal: 0,
        });
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            scrollbar_area,
            &mut self.scrollbar_state,
        );

        let hint = Paragraph::new("[ESC] Back to Chat");
        f.render_widget(hint, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent, _ctx: &AppContext) -> Option<Action> {
        match key.code {
            KeyCode::Up => {
                if self.state.scroll_offset > 0 {
                    self.state.scroll_offset -= 1;
                }
                Some(Action::ScrollDebugUp)
            }
            KeyCode::Down => {
                let max_scroll = self.state.content_height.saturating_sub(1);
                if self.state.scroll_offset < max_scroll {
                    self.state.scroll_offset += 1;
                }
                Some(Action::ScrollDebugDown)
            }
            KeyCode::Esc => Some(Action::SwitchView(crate::tui::ViewState::Chat(
                crate::tui::ChatViewState::new("deepseek", "deepseek-chat"),
            ))),
            _ => None,
        }
    }
}
