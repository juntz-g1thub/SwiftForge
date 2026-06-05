use swiftforge::tui::{ConfigContext, ConfigView, View};

#[test]
fn test_config_view_initial_state() {
    let view = ConfigView::new();
    assert!(view.state.editing_provider.is_none());
}

#[test]
fn test_config_view_state_editing_provider() {
    let mut view = ConfigView::new();
    assert!(view.state.editing_provider.is_none());

    view.state.editing_provider = Some("openai".to_string());
    assert_eq!(view.state.editing_provider, Some("openai".to_string()));
}

#[test]
fn test_config_context_default() {
    let ctx = ConfigContext::default();
    assert!(ctx.editing_provider.is_none());
}

#[test]
fn test_config_context_editing() {
    let mut ctx = ConfigContext::default();
    ctx.editing_provider = Some("anthropic".to_string());
    assert_eq!(ctx.editing_provider, Some("anthropic".to_string()));
}
