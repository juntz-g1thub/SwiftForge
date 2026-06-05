use swiftforge::tui::{ChatView, ChatViewState};

#[test]
fn test_chat_view_initial_state() {
    let view = ChatView::new("openai", "gpt-4o");
    assert!(view.state.input.is_empty());
    assert_eq!(view.state.cursor_pos, 0);
    assert!(!view.state.is_streaming);
    assert!(view.state.messages.is_empty());
}

#[test]
fn test_chat_view_state_provider_info() {
    let view = ChatView::new("anthropic", "claude-3-5-sonnet");
    assert_eq!(view.state.current_provider, "anthropic");
    assert_eq!(view.state.current_model, "claude-3-5-sonnet");
}

#[test]
fn test_chat_view_state_add_message() {
    let mut view = ChatView::new("openai", "gpt-4o");
    view.state.add_message("user", "Hello");
    view.state.add_message("assistant", "Hi there!");

    assert_eq!(view.state.messages.len(), 2);
    assert_eq!(view.state.messages[0].0, "user");
    assert_eq!(view.state.messages[0].1, "Hello");
    assert_eq!(view.state.messages[1].0, "assistant");
    assert_eq!(view.state.messages[1].1, "Hi there!");
}

#[test]
fn test_chat_view_state_streaming_flag() {
    let mut view = ChatView::new("openai", "gpt-4o");
    assert!(!view.state.is_streaming);

    view.state.is_streaming = true;
    assert!(view.state.is_streaming);

    view.state.is_streaming = false;
    assert!(!view.state.is_streaming);
}

#[test]
fn test_chat_view_state_cursor_position() {
    let mut view = ChatView::new("openai", "gpt-4o");

    view.state.input = "Hello".to_string();
    view.state.cursor_pos = 3;
    assert_eq!(view.state.cursor_pos, 3);

    view.state.cursor_pos = 5;
    assert_eq!(view.state.cursor_pos, 5);
}

#[test]
fn test_chat_view_state_scroll_offset() {
    let mut view = ChatView::new("openai", "gpt-4o");

    for i in 0..20 {
        view.state.add_message("user", &format!("Message {}", i));
    }

    assert_eq!(view.state.scroll_offset, 0);

    view.state.scroll_offset = 10;
    assert_eq!(view.state.scroll_offset, 10);

    view.state.scroll_offset = 19;
    assert_eq!(view.state.scroll_offset, 19);
}

#[test]
fn test_chat_view_state_content_height_not_updated_by_add_message() {
    let mut view = ChatView::new("openai", "gpt-4o");

    for i in 0..5 {
        view.state.add_message("user", &format!("Line {}", i));
    }

    assert_eq!(view.state.content_height, 0);
}

#[test]
fn test_chat_view_state_message_roles() {
    let mut view = ChatView::new("openai", "gpt-4o");
    view.state.add_message("user", "User message");
    view.state.add_message("assistant", "Assistant message");
    view.state.add_message("system", "System message");
    view.state.add_message("error", "Error message");

    assert_eq!(view.state.messages.len(), 4);
    assert_eq!(view.state.messages[0].0, "user");
    assert_eq!(view.state.messages[1].0, "assistant");
    assert_eq!(view.state.messages[2].0, "system");
    assert_eq!(view.state.messages[3].0, "error");
}

#[test]
fn test_chat_view_state_empty_input() {
    let view = ChatView::new("openai", "gpt-4o");
    assert!(view.state.input.is_empty());
}

#[test]
fn test_chat_view_state_multiple_messages() {
    let mut view = ChatView::new("openai", "gpt-4o");

    for i in 0..100 {
        view.state.add_message("user", &format!("Message {}", i));
    }

    assert_eq!(view.state.messages.len(), 100);
    assert_eq!(view.state.messages[99].1, "Message 99");
}

#[test]
fn test_chat_view_state_with_different_providers() {
    let providers = vec![
        ("openai", "gpt-4o"),
        ("anthropic", "claude-3-5-sonnet"),
        ("deepseek", "deepseek-v4-flash"),
        ("ollama", "llama3"),
    ];

    for (provider, model) in providers {
        let view = ChatView::new(provider, model);
        assert_eq!(view.state.current_provider, provider);
        assert_eq!(view.state.current_model, model);
    }
}
