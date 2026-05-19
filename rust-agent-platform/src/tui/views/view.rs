use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

use crate::tui::state::{Action, AppContext, UIState};

pub trait View {
    fn render(&mut self, f: &mut Frame, area: Rect, ctx: &AppContext, ui_state: &UIState);
    fn handle_key(&mut self, key: KeyEvent, ctx: &AppContext) -> Option<Action>;
    fn on_enter(&mut self) {}
    fn on_exit(&mut self) {}
}
