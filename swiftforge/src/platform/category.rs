use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IntentCategory {
    Research,
    Implementation,
    Investigation,
    Evaluation,
    Fix,
    OpenEnded,
    Trivial,
}

impl IntentCategory {
    pub fn from_str(s: &str) -> Self {
        let lower = s.to_lowercase();
        if lower.contains("implement") || lower.contains("add ") || lower.contains("create") {
            IntentCategory::Implementation
        } else if lower.contains("explain")
            || lower.contains("how does")
            || lower.contains("what is")
        {
            IntentCategory::Research
        } else if lower.contains("look into")
            || lower.contains("check ")
            || lower.contains("investigate")
        {
            IntentCategory::Investigation
        } else if lower.contains("what do you think") || lower.contains("evaluate ") {
            IntentCategory::Evaluation
        } else if lower.contains("improve")
            || lower.contains("refactor")
            || lower.contains("clean up")
        {
            IntentCategory::OpenEnded
        } else if lower.contains("error ") || lower.contains("broken") || lower.contains("fix ") {
            IntentCategory::Fix
        } else {
            IntentCategory::Trivial
        }
    }
}
