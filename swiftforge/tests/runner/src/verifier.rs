use crate::{
    parser::{ChatExpect, ToolExpect},
    StepResult, TestContext,
};
use anyhow::{anyhow, Result};
use swiftforge_types::ModelResponse;

pub struct Verifier;

impl Verifier {
    pub fn verify_chat_expect(response: &ModelResponse, expect: &ChatExpect) -> Result<()> {
        if expect.response_success && response.content.trim().is_empty() {
            return Err(anyhow!("Expected non-empty response but got empty content"));
        }

        if let Some(ref expected_text) = expect.response_contains {
            if !response.content.contains(expected_text) {
                return Err(anyhow!(
                    "Response does not contain expected text '{}'. Got: {}",
                    expected_text,
                    response.content
                ));
            }
        }

        if let Some(ref pattern) = expect.response_matches {
            let regex = regex::Regex::new(pattern)?;
            if !regex.is_match(&response.content) {
                return Err(anyhow!(
                    "Response does not match pattern '{}'. Got: {}",
                    pattern,
                    response.content
                ));
            }
        }

        Ok(())
    }

    pub fn verify_tool_expect(
        result: &swiftforge_types::ToolResult,
        expect: &ToolExpect,
    ) -> Result<()> {
        if expect.success != result.success {
            return Err(anyhow!(
                "Tool success mismatch: expected {}, got {}",
                expect.success,
                result.success
            ));
        }

        if let Some(ref expected_output) = expect.output_contains {
            let actual_output = result.output.as_deref().unwrap_or("");
            if !actual_output.contains(expected_output) {
                return Err(anyhow!(
                    "Tool output does not contain '{}'. Got: {}",
                    expected_output,
                    actual_output
                ));
            }
        }

        if expect.stderr_empty.unwrap_or(false) {
            if let Some(ref error) = result.error {
                if !error.trim().is_empty() {
                    return Err(anyhow!("Expected empty stderr but got: {}", error));
                }
            }
        }

        if let Some(ref expected_error) = expect.error_contains {
            let actual_error = result.error.as_deref().unwrap_or("");
            if !actual_error.contains(expected_error) {
                return Err(anyhow!(
                    "Tool error does not contain '{}'. Got: {}",
                    expected_error,
                    actual_error
                ));
            }
        }

        Ok(())
    }

    pub fn evaluate_condition(condition: &str, ctx: &TestContext) -> bool {
        let condition = condition.trim();

        if let Some(content) = ctx.get_last_response_content() {
            if condition.contains("response.content contains") {
                let parts: Vec<&str> = condition.split("response.content contains").collect();
                if parts.len() == 2 {
                    let expected = parts[1].trim().trim_matches('\'');
                    return content.contains(expected);
                }
            }
            if condition.contains("response.content") && condition.contains("error") {
                return content.to_lowercase().contains("error");
            }
        }

        if condition.contains("last_response.success == true") {
            return ctx.last_response.is_some();
        }
        if condition.contains("last_response.success == false") {
            return ctx.last_response.is_none();
        }

        if condition.starts_with("len(last_response.content)") {
            if let Some(ref response) = ctx.last_response {
                let parts: Vec<&str> = condition.split(" > ").collect();
                if parts.len() == 2 {
                    if let Some(len_part) = parts[0].split("len(last_response.content)").nth(1) {
                        let expected_len: usize = len_part.trim().parse().unwrap_or(0);
                        return response.content.len() > expected_len;
                    }
                }
            }
            return false;
        }

        if condition.contains("tool_results[0].success == true") {
            return ctx.tool_results.first().map(|r| r.success).unwrap_or(false);
        }
        if condition.contains("tool_results[0].success == false") {
            return ctx.tool_results.first().map(|r| !r.success).unwrap_or(true);
        }

        false
    }

    pub fn verify_final_state(
        ctx: &TestContext,
        messages_count: Option<usize>,
        last_role: Option<&str>,
        tools_called: Option<&[String]>,
    ) -> Result<()> {
        if let Some(expected_count) = messages_count {
            let actual_count = ctx.messages.len();
            if actual_count != expected_count {
                return Err(anyhow!(
                    "Message count mismatch: expected {}, got {}",
                    expected_count,
                    actual_count
                ));
            }
        }

        if let Some(expected_role) = last_role {
            if let Some(last_msg) = ctx.messages.last() {
                if &last_msg.role != expected_role {
                    return Err(anyhow!(
                        "Last message role mismatch: expected {}, got {}",
                        expected_role,
                        last_msg.role
                    ));
                }
            }
        }

        if let Some(expected_tools) = tools_called {
            let actual_tools: Vec<String> = ctx
                .tool_results
                .iter()
                .filter_map(|r| r.output.as_ref().map(|o| o.clone()))
                .collect();

            for expected_tool in expected_tools {
                if !actual_tools.iter().any(|t| t.contains(expected_tool)) {
                    return Err(anyhow!(
                        "Tool '{}' was not called during test",
                        expected_tool
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_condition_success() {
        let ctx = TestContext::new(HashMap::new());
        assert!(Verifier::evaluate_condition(
            "last_response.success == true",
            &ctx
        ));
    }

    #[test]
    fn test_evaluate_condition_failure() {
        let ctx = TestContext::new(HashMap::new());
        assert!(Verifier::evaluate_condition(
            "last_response.success == false",
            &ctx
        ));
    }
}
