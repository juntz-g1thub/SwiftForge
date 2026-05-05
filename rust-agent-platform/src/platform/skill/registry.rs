use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::loader::Skill;

#[derive(Clone)]
pub struct RegisteredSkill {
    pub skill: Arc<Skill>,
    pub enabled: bool,
}

pub struct SkillRegistry {
    skills: RwLock<HashMap<String, RegisteredSkill>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, skill: Skill) {
        let mut skills = self.skills.write().await;
        skills.insert(
            skill.name.clone(),
            RegisteredSkill {
                skill: Arc::new(skill),
                enabled: true,
            },
        );
    }

    pub async fn get(&self, name: &str) -> Option<Arc<Skill>> {
        let skills = self.skills.read().await;
        skills.get(name).map(|r| r.skill.clone())
    }

    pub async fn enable(&self, name: &str) -> bool {
        let mut skills = self.skills.write().await;
        if let Some(skill) = skills.get_mut(name) {
            skill.enabled = true;
            true
        } else {
            false
        }
    }

    pub async fn disable(&self, name: &str) -> bool {
        let mut skills = self.skills.write().await;
        if let Some(skill) = skills.get_mut(name) {
            skill.enabled = false;
            true
        } else {
            false
        }
    }

    pub async fn list_skills(&self) -> Vec<String> {
        let skills = self.skills.read().await;
        skills.keys().cloned().collect()
    }

    pub async fn list_enabled(&self) -> Vec<String> {
        let skills = self.skills.read().await;
        skills
            .iter()
            .filter(|(_, s)| s.enabled)
            .map(|(k, _)| k.clone())
            .collect()
    }
}