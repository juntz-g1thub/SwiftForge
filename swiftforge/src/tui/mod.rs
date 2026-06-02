mod components;
mod config;
mod state;
mod task;
mod views;

pub use config::{AppConfig, ConfigManager};
pub use state::{
    Action, AppContext, ChatContext, ChatViewState, ConfigContext, ConfigViewState, UIState,
    ViewState, ViewStateKind,
};
pub use views::{ChatView, ConfigView, View};

pub mod app_controller;
