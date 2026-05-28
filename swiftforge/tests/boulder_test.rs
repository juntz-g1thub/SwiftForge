use rust_agent_platform::platform::boulder::{BoulderPriority, BoulderStatus, BoulderStore};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn test_data_dir() -> PathBuf {
    let mut dir = std::env::temp_dir();
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    dir.push(format!("test_boulder_{}_{}", counter, std::process::id()));
    dir
}

fn setup_dir(dir: &PathBuf) {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
}

fn cleanup(dir: &PathBuf) {
    std::thread::sleep(std::time::Duration::from_millis(50));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_boulder_store_creation() {
    let dir = test_data_dir();
    setup_dir(&dir);
    let store = BoulderStore::new(dir.clone());
    cleanup(&dir);
    assert!(store.is_ok());
}

#[test]
fn test_create_boulder() {
    let dir = test_data_dir();
    setup_dir(&dir);
    let store = BoulderStore::new(dir.clone()).unwrap();

    let boulder = store
        .create(
            "Test task".to_string(),
            BoulderPriority::High,
            vec!["test".to_string()],
        )
        .unwrap();

    assert_eq!(boulder.content, "Test task");
    assert_eq!(boulder.status, BoulderStatus::Pending);
    assert_eq!(boulder.priority, BoulderPriority::High);

    cleanup(&dir);
}

#[test]
fn test_load_boulder() {
    let dir = test_data_dir();
    setup_dir(&dir);
    let store = BoulderStore::new(dir.clone()).unwrap();

    let created = store
        .create("Test task".to_string(), BoulderPriority::Medium, vec![])
        .unwrap();

    let loaded = store.load(&created.id).unwrap().unwrap();
    assert_eq!(loaded.content, "Test task");

    cleanup(&dir);
}

#[test]
fn test_list_boulders() {
    let dir = test_data_dir();
    setup_dir(&dir);
    let store = BoulderStore::new(dir.clone()).unwrap();

    store
        .create("Task 1".to_string(), BoulderPriority::Low, vec![])
        .unwrap();
    store
        .create("Task 2".to_string(), BoulderPriority::High, vec![])
        .unwrap();

    let list = store.list().unwrap();
    assert_eq!(list.len(), 2);

    cleanup(&dir);
}

#[test]
fn test_update_status() {
    let dir = test_data_dir();
    setup_dir(&dir);
    let store = BoulderStore::new(dir.clone()).unwrap();

    let boulder = store
        .create("Test".to_string(), BoulderPriority::Medium, vec![])
        .unwrap();
    store
        .update_status(&boulder.id, BoulderStatus::Completed)
        .unwrap();

    let updated = store.load(&boulder.id).unwrap().unwrap();
    assert_eq!(updated.status, BoulderStatus::Completed);

    cleanup(&dir);
}

#[test]
fn test_delete_boulder() {
    let dir = test_data_dir();
    setup_dir(&dir);
    let store = BoulderStore::new(dir.clone()).unwrap();

    let boulder = store
        .create("Test".to_string(), BoulderPriority::Medium, vec![])
        .unwrap();
    store.delete(&boulder.id).unwrap();

    let result = store.load(&boulder.id).unwrap();
    assert!(result.is_none());

    cleanup(&dir);
}
