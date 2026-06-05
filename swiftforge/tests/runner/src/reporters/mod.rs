use crate::{StepResult, TestScript, TestSummary};

pub trait Reporter: Send + Sync {
    fn on_test_start(&mut self, script: &TestScript);
    fn on_step_start(&mut self, step_name: &str, action: &str);
    fn on_step_complete(&mut self, result: &StepResult);
    fn on_test_complete(&mut self, summary: &TestSummary);
    fn finalize(&mut self) -> anyhow::Result<()>;
}

pub struct JsonReporter {
    output_path: Option<std::path::PathBuf>,
    results: Vec<TestSummary>,
}

impl JsonReporter {
    pub fn new(output_path: Option<std::path::PathBuf>) -> Self {
        Self {
            output_path,
            results: Vec::new(),
        }
    }
}

impl Reporter for JsonReporter {
    fn on_test_start(&mut self, _script: &TestScript) {}

    fn on_step_start(&mut self, _step_name: &str, _action: &str) {}

    fn on_step_complete(&mut self, _result: &StepResult) {}

    fn on_test_complete(&mut self, summary: &TestSummary) {
        self.results.push(summary.clone());
    }

    fn finalize(&mut self) -> anyhow::Result<()> {
        if let Some(ref path) = self.output_path {
            let json = serde_json::to_string_pretty(&self.results)?;
            std::fs::write(path, json)?;
        }
        Ok(())
    }
}

pub struct HtmlReporter {
    output_path: std::path::PathBuf,
    test_name: String,
    step_results: Vec<StepResult>,
    status: String,
    duration_ms: u64,
}

impl HtmlReporter {
    pub fn new(output_path: std::path::PathBuf) -> Self {
        Self {
            output_path,
            test_name: String::new(),
            step_results: Vec::new(),
            status: "running".to_string(),
            duration_ms: 0,
        }
    }

    fn generate_html(&self) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<meta charset=\"utf-8\">\n");
        html.push_str("<title>Test Report - ");
        html.push_str(&self.test_name);
        html.push_str("</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str(".passed { color: green; }\n");
        html.push_str(".failed { color: red; }\n");
        html.push_str(".skipped { color: orange; }\n");
        html.push_str("table { border-collapse: collapse; width: 100%; }\n");
        html.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
        html.push_str("th { background-color: #f2f2f2; }\n");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");
        html.push_str("<h1>Test Report</h1>\n");
        html.push_str("<p>Test: ");
        html.push_str(&self.test_name);
        html.push_str("</p>\n");
        html.push_str("<p>Status: <span class=\"");
        html.push_str(&self.status);
        html.push_str("\">");
        html.push_str(&self.status);
        html.push_str("</span></p>\n");
        html.push_str("<p>Duration: ");
        html.push_str(&format!("{} ms", self.duration_ms));
        html.push_str("</p>\n");
        html.push_str("<table>\n");
        html.push_str(
            "<tr><th>Step</th><th>Action</th><th>Status</th><th>Duration (ms)</th></tr>\n",
        );
        for result in &self.step_results {
            html.push_str("<tr>");
            html.push_str("<td>");
            html.push_str(&result.name);
            html.push_str("</td>");
            html.push_str("<td>");
            html.push_str(&result.action);
            html.push_str("</td>");
            html.push_str("<td class=\"");
            html.push_str(if result.success { "passed" } else { "failed" });
            html.push_str("\">");
            html.push_str(if result.success { "PASSED" } else { "FAILED" });
            html.push_str("</td>");
            html.push_str("<td>");
            html.push_str(&format!("{}", result.duration_ms));
            html.push_str("</td>");
            html.push_str("</tr>\n");
        }
        html.push_str("</table>\n");
        html.push_str("</body>\n</html>\n");
        html
    }
}

impl Reporter for HtmlReporter {
    fn on_test_start(&mut self, script: &TestScript) {
        self.test_name = script.name.clone();
    }

    fn on_step_start(&mut self, _step_name: &str, _action: &str) {}

    fn on_step_complete(&mut self, result: &StepResult) {
        self.step_results.push(result.clone());
    }

    fn on_test_complete(&mut self, summary: &TestSummary) {
        self.status = summary.status.to_string();
        self.duration_ms = summary.duration_ms;
    }

    fn finalize(&mut self) -> anyhow::Result<()> {
        let html = self.generate_html();
        std::fs::write(&self.output_path, html)?;
        Ok(())
    }
}

pub struct JunitReporter {
    output_path: std::path::PathBuf,
    testsuites: Vec<TestSuite>,
}

#[derive(serde::Serialize)]
struct TestSuite {
    name: String,
    tests: usize,
    failures: usize,
    time: String,
    testcases: Vec<TestCase>,
}

#[derive(serde::Serialize)]
struct TestCase {
    name: String,
    classname: String,
    time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<Failure>,
}

#[derive(serde::Serialize)]
struct Failure {
    message: String,
    #[serde(rename = "type")]
    failure_type: String,
}

impl JunitReporter {
    pub fn new(output_path: std::path::PathBuf) -> Self {
        Self {
            output_path,
            testsuites: Vec::new(),
        }
    }
}

impl Reporter for JunitReporter {
    fn on_test_start(&mut self, _script: &TestScript) {}

    fn on_step_start(&mut self, _step_name: &str, _action: &str) {}

    fn on_step_complete(&mut self, _result: &StepResult) {}

    fn on_test_complete(&mut self, summary: &TestSummary) {
        let mut testcases = Vec::new();
        for result in &summary.step_results {
            let failure = if result.success {
                None
            } else {
                Some(Failure {
                    message: result
                        .error
                        .clone()
                        .unwrap_or_else(|| "Unknown error".to_string()),
                    failure_type: "AssertionError".to_string(),
                })
            };
            testcases.push(TestCase {
                name: result.name.clone(),
                classname: "test".to_string(),
                time: format!(
                    "{}.{:03}",
                    result.duration_ms / 1000,
                    result.duration_ms % 1000
                ),
                failure,
            });
        }

        let failures = summary.failed_steps;
        self.testsuites.push(TestSuite {
            name: summary.name.clone(),
            tests: summary.total_steps,
            failures,
            time: format!(
                "{}.{:03}",
                summary.duration_ms / 1000,
                summary.duration_ms % 1000
            ),
            testcases,
        });
    }

    fn finalize(&mut self) -> anyhow::Result<()> {
        let xml = serde_xml_rs::to_string(&self.testsuites)
            .map_err(|e| anyhow::anyhow!("Failed to generate JUnit XML: {}", e))?;
        std::fs::write(&self.output_path, xml)?;
        Ok(())
    }
}
