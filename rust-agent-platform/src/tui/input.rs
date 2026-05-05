use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct InputHandler;

impl InputHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_key(&self, key: KeyEvent) -> Option<InputAction> {
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                    Some(InputAction::Quit)
                } else if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    Some(InputAction::Insert(c))
                } else {
                    None
                }
            }
            KeyCode::Backspace => Some(InputAction::Backspace),
            KeyCode::Enter => Some(InputAction::Submit),
            _ => None,
        }
    }
}

pub enum InputAction {
    Insert(char),
    Backspace,
    Submit,
    Quit,
}
