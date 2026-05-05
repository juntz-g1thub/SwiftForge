use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub commands: Vec<SkillCommand>,
    pub scope: SkillScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCommand {
    pub name: String,
    pub description: Option<String>,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillScope {
    Global,
    Project,
    User,
}

pub struct SkillLoader;

impl SkillLoader {
    pub fn new() -> Self {
        Self
    }

    pub fn load_skill(&self, path: &Path) -> Result<Skill> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read skill file: {:?}", path))?;

        self.parse_skill(&content)
    }

    pub fn parse_skill(&self, content: &str) -> Result<Skill> {
        let trimmed = content.trim();
        let frontmatter = if trimmed.starts_with("---") {
            trimmed
                .lines()
                .skip(1)
                .take_while(|l| !l.starts_with("---"))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            trimmed
                .lines()
                .take_while(|l| {
                    !l.is_empty()
                        && (l.starts_with("name:")
                            || l.starts_with("description:")
                            || l.starts_with("scope:"))
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let name = self
            .extract_field(&frontmatter, "name")
            .unwrap_or_else(|| "Unknown Skill".to_string());
        let description = self
            .extract_field(&frontmatter, "description")
            .unwrap_or_else(|| "No description".to_string());
        let scope_str = self
            .extract_field(&frontmatter, "scope")
            .unwrap_or_else(|| "Project".to_string());

        let scope = match scope_str.as_str() {
            "Global" => SkillScope::Global,
            "User" => SkillScope::User,
            _ => SkillScope::Project,
        };

        Ok(Skill {
            name,
            description,
            commands: Vec::new(),
            scope,
        })
    }

    fn extract_field(&self, content: &str, field: &str) -> Option<String> {
        for line in content.lines() {
            if line.starts_with(&format!("{}:", field)) {
                let value = line.split(':').nth(1)?.trim();
                return Some(value.to_string());
            }
        }
        None
    }

    pub fn load_from_directory(&self, dir: &Path) -> Result<Vec<Skill>> {
        let mut skills = Vec::new();

        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "md") {
                    if let Ok(skill) = self.load_skill(&path) {
                        skills.push(skill);
                    }
                }
            }
        }

        Ok(skills)
    }
}
