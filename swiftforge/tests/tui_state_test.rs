use swiftforge::tui::{
    Action, ChatContext, ChatViewState, ConfigContext, ConfigViewState, StreamingState, ViewState,
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
    assert_eq!(state.streaming_state, StreamingState::Idle);
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
fn test_chat_view_state_streaming_state() {
    let mut state = ChatViewState::new("openai", "gpt-4o");
    assert_eq!(state.streaming_state, StreamingState::Idle);
    assert!(!state.streaming_state.is_active());
    assert!(!state.streaming_state.is_terminal());

    state.streaming_state = StreamingState::Streaming;
    assert_eq!(state.streaming_state, StreamingState::Streaming);
    assert!(state.streaming_state.is_active());
    assert!(!state.streaming_state.is_terminal());

    state.streaming_state = StreamingState::Completed;
    assert_eq!(state.streaming_state, StreamingState::Completed);
    assert!(!state.streaming_state.is_active());
    assert!(state.streaming_state.is_terminal());

    state.streaming_state = StreamingState::Error("test error".to_string());
    assert!(state.streaming_state.is_terminal());
    assert!(!state.streaming_state.is_active());
}

#[test]
fn test_streaming_pipeline_data_flow() {
    // Simulate the full streaming pipeline:
    // SendMessage → Streaming → chunk accumulation → Completed → add_message
    let mut state = ChatViewState::new("openai", "gpt-4o");

    // Phase 1: User sends message
    state.add_message("user", "Hello");
    assert_eq!(state.messages.len(), 1);

    // Phase 2: Streaming starts
    state.streaming_state = StreamingState::Streaming;
    assert!(state.streaming_state.is_active());

    // Phase 3: Chunks accumulate (simulating streaming_text)
    let mut streaming_text: Option<String> = None;
    for chunk in &["Hello", " world", " from", " streaming"] {
        if let Some(ref mut text) = streaming_text {
            text.push_str(chunk);
        } else {
            streaming_text = Some(chunk.to_string());
        }
    }
    assert_eq!(
        streaming_text.as_deref(),
        Some("Hello world from streaming")
    );

    // Phase 4: Finalization — migrate streaming_text to messages
    if let Some(text) = streaming_text.take() {
        if !text.is_empty() {
            state.add_message("assistant", &text);
            state.streaming_state = StreamingState::Completed;
        }
    }

    // Verify final state
    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[1].0, "assistant");
    assert_eq!(state.messages[1].1, "Hello world from streaming");
    assert_eq!(state.streaming_state, StreamingState::Completed);
    assert!(!state.streaming_state.is_active());
}

#[test]
fn test_streaming_cancellation() {
    // Simulate user cancelling streaming mid-flight
    let mut state = ChatViewState::new("openai", "gpt-4o");

    state.add_message("user", "Hello");
    state.streaming_state = StreamingState::Streaming;

    // Cancel: clear streaming, set to Idle
    state.streaming_state = StreamingState::Idle;

    assert_eq!(state.streaming_state, StreamingState::Idle);
    assert!(!state.streaming_state.is_active());
    assert_eq!(state.messages.len(), 1); // Only user message, no assistant
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
