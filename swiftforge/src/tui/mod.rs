mod components;
pub mod config;
mod state;
pub mod task;
mod views;

pub use config::{AppConfig, ConfigManager, ProviderSettings};
pub use state::{
    Action, AppContext, ChatContext, ChatViewState, ConfigContext, ConfigViewState, UIState,
    ViewState, ViewStateKind,
};
pub use views::{ChatView, ConfigView, View};

pub mod app_controller;
