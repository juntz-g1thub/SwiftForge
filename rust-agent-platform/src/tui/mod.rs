mod views;
mod state;
mod components;
mod config;

pub use config::{AppConfig, ConfigManager};
pub use views::{View, ChatView, ConfigView, DebugView};
pub use state::{Action, ViewState, ViewStateKind, AppContext, UIState, ChatViewState, ConfigViewState, DebugViewState};

pub mod app_controller;