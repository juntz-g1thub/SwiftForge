use crate::platform::category::IntentCategory;

pub struct IntentGate;

impl IntentGate {
    pub fn new() -> Self {
        Self
    }

    pub fn classify(&self, input: &str) -> IntentCategory {
        IntentCategory::from_str(input)
    }

    pub fn classify_with_confidence(&self, input: &str) -> (IntentCategory, f32) {
        let category = self.classify(input);
        let confidence = self.calculate_confidence(input, &category);
        (category, confidence)
    }

    fn calculate_confidence(&self, input: &str, category: &IntentCategory) -> f32 {
        let lower = input.to_lowercase();
        let mut score: f32 = 0.5;

        match category {
            IntentCategory::Implementation => {
                if lower.contains("implement") || lower.contains("add") || lower.contains("create")
                {
                    score += 0.3;
                }
            }
            IntentCategory::Research => {
                if lower.contains("explain")
                    || lower.contains("how does")
                    || lower.contains("what is")
                {
                    score += 0.3;
                }
            }
            IntentCategory::Investigation => {
                if lower.contains("look into")
                    || lower.contains("check")
                    || lower.contains("investigate")
                {
                    score += 0.3;
                }
            }
            IntentCategory::Evaluation => {
                if lower.contains("what do you think") || lower.contains("evaluate") {
                    score += 0.3;
                }
            }
            IntentCategory::Fix => {
                if lower.contains("error") || lower.contains("broken") || lower.contains("fix") {
                    score += 0.3;
                }
            }
            IntentCategory::OpenEnded => {
                if lower.contains("improve")
                    || lower.contains("refactor")
                    || lower.contains("clean")
                {
                    score += 0.3;
                }
            }
            IntentCategory::Trivial => {
                score = 0.8;
            }
        }

        score.min(1.0)
    }

    pub fn route_hint(&self, category: &IntentCategory) -> &str {
        match category {
            IntentCategory::Research => "explore/librarian → synthesize → answer",
            IntentCategory::Implementation => "plan → delegate or execute",
            IntentCategory::Investigation => "explore → report findings",
            IntentCategory::Evaluation => "evaluate → propose → wait for confirmation",
            IntentCategory::Fix => "diagnose → fix minimally",
            IntentCategory::OpenEnded => "assess codebase first → propose approach",
            IntentCategory::Trivial => "direct tools only",
        }
    }
}
