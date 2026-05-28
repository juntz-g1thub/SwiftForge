use rust_agent_platform::platform::{IntentCategory, IntentGate};

#[test]
fn test_classify_implementation() {
    let gate = IntentGate::new();
    let result = gate.classify("implement user authentication");
    assert_eq!(result, IntentCategory::Implementation);
}

#[test]
fn test_classify_research() {
    let gate = IntentGate::new();
    let result = gate.classify("how does async/await work");
    assert_eq!(result, IntentCategory::Research);
}

#[test]
fn test_classify_investigation() {
    let gate = IntentGate::new();
    let result = gate.classify("look into the memory leak issue");
    assert_eq!(result, IntentCategory::Investigation);
}

#[test]
fn test_classify_evaluation() {
    let gate = IntentGate::new();
    let result = gate.classify("what do you think about this design");
    assert_eq!(result, IntentCategory::Evaluation);
}

#[test]
fn test_classify_fix() {
    let gate = IntentGate::new();
    let result = gate.classify("I'm seeing error E1234");
    assert_eq!(result, IntentCategory::Fix);
}

#[test]
fn test_classify_open_ended() {
    let gate = IntentGate::new();
    let result = gate.classify("improve the error handling");
    assert_eq!(result, IntentCategory::OpenEnded);
}

#[test]
fn test_classify_trivial() {
    let gate = IntentGate::new();
    let result = gate.classify("what time is it");
    assert_eq!(result, IntentCategory::Trivial);
}

#[test]
fn test_route_hint() {
    let gate = IntentGate::new();
    let hint = gate.route_hint(&IntentCategory::Implementation);
    assert!(hint.contains("delegate"));
}
