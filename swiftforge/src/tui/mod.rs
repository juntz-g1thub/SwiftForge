mod components;
pub mod config;
mod initializer;
mod state;
pub mod task;
mod views;

pub use config::{AppConfig, ConfigManager, ProviderSettings};
pub use state::{
    Action, AppContext, ChatContext, ChatViewState, ConfigContext, ConfigViewState, StreamingState,
    UIState, ViewState, ViewStateKind,
};
pub use views::{ChatView, ConfigView, View};

pub mod app_controller;
