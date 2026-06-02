use rust_agent_platform::platform::hooks::{HookContext, HookEvent, HookRegistry};
use std::sync::Arc;

#[tokio::test]
async fn test_hook_registry_creation() {
    let registry = HookRegistry::new();
    let hooks = registry.list_hooks().await;
    assert!(hooks.is_empty());
}

#[tokio::test]
async fn test_hook_registration() {
    let registry = HookRegistry::new();
    let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count_clone = call_count.clone();

    let hook = Arc::new(move |_ctx: HookContext| {
        count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    });

    registry.register("OnStartup", 0, hook).await;
    let hooks = registry.list_hooks().await;
    assert!(hooks.contains(&"OnStartup".to_string()));
}

#[tokio::test]
async fn test_hook_dispatch() {
    let registry = HookRegistry::new();
    let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count_clone = call_count.clone();

    let hook = Arc::new(move |_ctx: HookContext| {
        count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    });

    registry.register("OnStartup", 0, hook).await;

    registry.dispatch(HookEvent::OnStartup).await.unwrap();

    assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_hook_context_creation() {
    let context = HookContext::new(HookEvent::OnStartup);
    assert!(format!("{:?}", context.event).contains("OnStartup"));
}
