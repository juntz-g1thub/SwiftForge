use swiftforge::tui::{
    Action, ChatContext, ChatViewState, ConfigContext, ConfigViewState, ViewState,
};

#[test]
fn test_chat_context_creation() {
    let ctx = ChatContext::new("openai", "gpt-4o");
    assert_eq!(ctx.current_provider, "openai");
    assert_eq!(ctx.current_model, "gpt-4o");
}

#[test]
fn test_chat_view_state_creation() {
    let state = ChatViewState::new("anthropic", "claude-3-5-sonnet");
    assert_eq!(state.current_provider, "anthropic");
    assert_eq!(state.current_model, "claude-3-5-sonnet");
    assert!(state.messages.is_empty());
    assert_eq!(state.input, "");
    assert_eq!(state.cursor_pos, 0);
    assert_eq!(state.scroll_offset, 0);
    assert!(!state.is_streaming);
}

#[test]
fn test_chat_view_state_add_message() {
    let mut state = ChatViewState::new("openai", "gpt-4o");
    state.add_message("user", "Hello");
    state.add_message("assistant", "Hi there!");

    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].0, "user");
    assert_eq!(state.messages[0].1, "Hello");
    assert_eq!(state.messages[1].0, "assistant");
    assert_eq!(state.messages[1].1, "Hi there!");
}

#[test]
fn test_config_view_state_default() {
    let state = ConfigViewState::new();
    assert!(state.editing_provider.is_none());
}

#[test]
fn test_view_state_as_chat() {
    let chat_ctx = ChatContext::new("openai", "gpt-4o");
    let view_state = ViewState::Chat(chat_ctx);

    assert!(view_state.as_chat().is_some());
    assert!(view_state.as_config().is_none());
}

#[test]
fn test_view_state_as_config() {
    let config_ctx = ConfigContext::default();
    let view_state = ViewState::Config(config_ctx);

    assert!(view_state.as_config().is_some());
    assert!(view_state.as_chat().is_none());
}

#[test]
fn test_view_state_as_chat_on_chat() {
    let chat_ctx = ChatContext::new("deepseek", "deepseek-v4-flash");
    let view_state = ViewState::Chat(chat_ctx);

    let retrieved = view_state.as_chat().unwrap();
    assert_eq!(retrieved.current_provider, "deepseek");
    assert_eq!(retrieved.current_model, "deepseek-v4-flash");
}

#[test]
fn test_action_variants() {
    // Test that Action variants exist and can be created
    let actions = vec![
        Action::Quit,
        Action::CancelStreaming,
        Action::ScrollUp,
        Action::ScrollDown,
    ];

    // Verify we can match on actions (compile-time check)
    for action in actions {
        match action {
            Action::Quit => {}
            Action::CancelStreaming => {}
            Action::ScrollUp => {}
            Action::ScrollDown => {}
            _ => {}
        }
    }
}

#[test]
fn test_chat_view_state_streaming_flag() {
    let mut state = ChatViewState::new("openai", "gpt-4o");
    assert!(!state.is_streaming);

    state.is_streaming = true;
    assert!(state.is_streaming);

    state.is_streaming = false;
    assert!(!state.is_streaming);
}

#[test]
fn test_chat_view_state_cursor_position() {
    let mut state = ChatViewState::new("openai", "gpt-4o");

    state.input = "Hello".to_string();
    state.cursor_pos = 3;
    assert_eq!(state.cursor_pos, 3);

    // Cursor at end
    state.cursor_pos = state.input.len();
    assert_eq!(state.cursor_pos, 5);
}

#[test]
fn test_chat_view_state_scroll_offset() {
    let mut state = ChatViewState::new("openai", "gpt-4o");

    // Add many messages
    for i in 0..20 {
        state.add_message("user", &format!("Message {}", i));
    }

    assert_eq!(state.messages.len(), 20);
    assert_eq!(state.scroll_offset, 0);

    // Scroll down
    state.scroll_offset = 10;
    assert_eq!(state.scroll_offset, 10);
}

#[test]
fn test_config_context_editing_provider() {
    let mut ctx = ConfigContext::default();
    assert!(ctx.editing_provider.is_none());

    ctx.editing_provider = Some("openai".to_string());
    assert_eq!(ctx.editing_provider, Some("openai".to_string()));
}
