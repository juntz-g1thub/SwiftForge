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
    Quit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewStateKind {
    Chat,
    Config,
}
