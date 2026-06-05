mod parser;
mod context;
mod verifier;
mod executor;
pub mod reporters;

use anyhow::Result;
use std::path::PathBuf;

pub use parser::{TestScript, TestStep, ScriptConfig, ProviderConfig, AgentScriptConfig, EnvironmentConfig};
pub use context::TestContext;
pub use verifier::Verifier;
pub use executor::TestExecutor;
pub use reporters::{Reporter, JsonReporter, HtmlReporter, JunitReporter};

#[derive(Debug, Clone, serde::Serialize)]
pub struct StepResult {
    pub name: String,
    pub action: String,
    pub success: bool,
    pub duration_ms: u64,
    pub response: Option<String>,
    pub tool_result: Option<String>,
    pub error: Option<String>,
    pub conditions: Vec<String>,
}

impl StepResult {
    pub fn success(name: &str, action: &str, duration_ms: u64) -> Self {
        Self {
            name: name.to_string(),
            action: action.to_string(),
            success: true,
            duration_ms,
            response: None,
            tool_result: None,
            error: None,
            conditions: Vec::new(),
        }
    }

    pub fn failure(name: &str, action: &str, error: String, duration_ms: u64) -> Self {
        Self {
            name: name.to_string(),
            action: action.to_string(),
            success: false,
            duration_ms,
            response: None,
            tool_result: None,
            error: Some(error),
            conditions: Vec::new(),
        }
    }

    pub fn with_response(mut self, response: String) -> Self {
        self.response = Some(response);
        self
    }

    pub fn with_tool_result(mut self, result: String) -> Self {
        self.tool_result = Some(result);
        self
    }

    pub fn with_conditions(mut self, conditions: Vec<String>) -> Self {
        self.conditions = conditions;
        self
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TestSummary {
    pub name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub total_steps: usize,
    pub passed_steps: usize,
    pub failed_steps: usize,
    pub skipped_steps: usize,
    pub step_results: Vec<StepResult>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "passed"),
            TestStatus::Failed => write!(f, "failed"),
            TestStatus::Skipped => write!(f, "skipped"),
            TestStatus::Error => write!(f, "error"),
        }
    }
}

impl TestSummary {
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: TestStatus::Passed,
            duration_ms: 0,
            total_steps: 0,
            passed_steps: 0,
            failed_steps: 0,
            skipped_steps: 0,
            step_results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: StepResult) {
        self.total_steps += 1;
        if result.success {
            self.passed_steps += 1;
        } else {
            self.failed_steps += 1;
            if self.status == TestStatus::Passed {
                self.status = TestStatus::Failed;
            }
        }
        self.duration_ms += result.duration_ms;
        self.step_results.push(result);
    }

    pub fn set_error(&mut self) {
        self.status = TestStatus::Error;
    }
}

pub fn load_script(path: &PathBuf) -> Result<TestScript> {
    let content = std::fs::read_to_string(path)?;
    parser::parse_script(&content)
}

pub fn load_scripts_from_dir(dir: &PathBuf) -> Result<Vec<TestScript>> {
    let mut scripts = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                match load_script(&path) {
                    Ok(script) => scripts.push(script),
                    Err(e) => tracing::warn!("Failed to parse script {:?}: {}", path, e),
                }
            }
        }
    }
    Ok(scripts)
}
