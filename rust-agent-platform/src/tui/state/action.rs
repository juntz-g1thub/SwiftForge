use super::view_state::ViewState;

#[derive(Debug, Clone)]
pub enum Action {
    SendMessage(String),
    CancelStreaming,
    AppendMessage(String, String),
    SwitchView(ViewState),
    GoBack,
    ScrollUp,
    ScrollDown,
    ScrollDebugUp,
    ScrollDebugDown,
    ResetScroll,
    InputChar(char),
    InputBackspace,
    InputDelete,
    InputHome,
    InputEnd,
    InputLeft,
    InputRight,
    ClearInput,
    SelectProvider(String),
    SaveApiKey(String),
    SaveModel(String),
    SaveBaseUrl(String),
    FetchModels,
    SelectModel(String),
    ToggleDebug,
    Quit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewStateKind {
    Chat,
    Config,
    Debug,
}
