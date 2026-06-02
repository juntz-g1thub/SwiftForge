use rust_agent_platform::platform::skill::{SkillLoader, SkillRegistry, SkillScope};
use std::path::PathBuf;

#[test]
fn test_skill_loader_creation() {
    let loader = SkillLoader::new();
}

#[test]
fn test_parse_skill_from_content() {
    let loader = SkillLoader::new();
    let content = r#"
---
name: test-skill
description: A test skill
scope: Project
---

# Test Skill

This is a test skill content.
"#;

    let skill = loader.parse_skill(content).unwrap();
    assert_eq!(skill.name, "test-skill");
    assert_eq!(skill.description, "A test skill");
    assert_eq!(skill.scope, SkillScope::Project);
}

#[test]
fn test_skill_registry_creation() {
    let registry = SkillRegistry::new();
}

#[tokio::test]
async fn test_skill_registry_register() {
    use rust_agent_platform::platform::skill::Skill;

    let registry = SkillRegistry::new();
    let skill = Skill {
        name: "test".to_string(),
        description: "Test skill".to_string(),
        commands: Vec::new(),
        scope: SkillScope::Project,
    };

    registry.register(skill).await;

    let skills = registry.list_skills().await;
    assert!(skills.contains(&"test".to_string()));
}

#[tokio::test]
async fn test_skill_registry_enable_disable() {
    use rust_agent_platform::platform::skill::Skill;

    let registry = SkillRegistry::new();
    let skill = Skill {
        name: "test".to_string(),
        description: "Test".to_string(),
        commands: Vec::new(),
        scope: SkillScope::Project,
    };

    registry.register(skill).await;
    assert!(registry.enable("test").await);
    assert!(registry.disable("test").await);
}
