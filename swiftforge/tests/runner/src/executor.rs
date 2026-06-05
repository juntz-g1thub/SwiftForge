use crate::{StepResult, TestContext, TestScript, TestSummary, TestStatus};
use crate::parser::{get_step_action, get_step_name, TestStep};
use crate::reporters::Reporter;
use crate::Verifier;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use swiftforge::{Agent, AgentConfig, AgentRole};
use swiftforge_providers::{DeepSeekProvider, OpenAIProvider};
use swiftforge_provider_core::DynLLMProvider;
use swiftforge_types::{Message, ModelResponse, ToolRegistry, ToolResult};
use tokio::sync::RwLock;

pub struct TestExecutor {
    script: TestScript,
    context: TestContext,
    reporters: Vec<Box<dyn Reporter>>,
    agent: Option<Agent>,
}

impl TestExecutor {
    pub fn new(script: TestScript, reporters: Vec<Box<dyn Reporter>>) -> Self {
        let env: HashMap<String, String> = std::env::vars().collect();
        Self {
            script,
            context: TestContext::new(env),
            reporters,
            agent: None,
        }
    }

    pub fn finalize(&mut self) -> Result<()> {
        for reporter in &mut self.reporters {
            reporter.finalize()?;
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<TestSummary> {
        let start_time = Instant::now();
        let mut summary = TestSummary::new(self.script.name.clone());

        for reporter in &mut self.reporters {
            reporter.on_test_start(&self.script);
        }

        self.initialize_agent().await?;

        let steps = self.script.steps.clone();
        for step in &steps {
            let step_name = get_step_name(step).to_string();
            let step_action = get_step_action(step).to_string();

            for reporter in &mut self.reporters {
                reporter.on_step_start(&step_name, &step_action);
            }

            let result = self.execute_step(step).await;

            match result {
                Ok(step_result) => {
                    let success = step_result.success;
                    for reporter in &mut self.reporters {
                        reporter.on_step_complete(&step_result);
                    }
                    summary.add_result(step_result);
                    if !success {
                        break;
                    }
                }
                Err(e) => {
                    let step_result = StepResult::failure(
                        &step_name,
                        &step_action,
                        e.to_string(),
                        0,
                    );
                    for reporter in &mut self.reporters {
                        reporter.on_step_complete(&step_result);
                    }
                    summary.add_result(step_result);
                    summary.set_error();
                    break;
                }
            }
        }

        if let Some(ref verify_config) = self.script.verify {
            if let Some(ref final_state) = verify_config.final_state {
                if let Err(e) = Verifier::verify_final_state(
                    &self.context,
                    final_state.messages_count,
                    final_state.last_role.as_deref(),
                    final_state.tools_called.as_deref(),
                ) {
                    tracing::warn!("Final state verification failed: {}", e);
                }
            }
        }

        summary.duration_ms = start_time.elapsed().as_millis() as u64;

        for reporter in &mut self.reporters {
            reporter.on_test_complete(&summary);
        }

        Ok(summary)
    }

    async fn initialize_agent(&mut self) -> Result<()> {
        let provider_config = &self.script.config.provider;
        let agent_config = &self.script.config.agent;

        let provider: swiftforge_provider_core::DynLLMProvider = match provider_config.provider_type.as_str() {
            "openai" => {
                let p = OpenAIProvider::new(
                    provider_config.api_key.clone(),
                    provider_config.base_url.clone(),
                );
                Arc::new(p) as swiftforge_provider_core::DynLLMProvider
            }
            "deepseek" => {
                let p = DeepSeekProvider::new(
                    provider_config.api_key.clone(),
                    provider_config.base_url.clone(),
                    provider_config.model.clone(),
                );
                Arc::new(p) as swiftforge_provider_core::DynLLMProvider
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported provider type: {}", provider_config.provider_type));
            }
        };

        let role = match agent_config.role.as_str() {
            "Orchestrator" => AgentRole::Orchestrator,
            "Executor" => AgentRole::Executor,
            "Planner" => AgentRole::Planner,
            "Advisor" => AgentRole::Advisor,
            "Explorer" => AgentRole::Explorer,
            "Librarian" => AgentRole::Librarian,
            _ => AgentRole::Executor,
        };

        let config = AgentConfig {
            name: agent_config.name.clone(),
            role,
            model: provider_config.model.clone(),
            temperature: provider_config.temperature.unwrap_or(0.0),
        };

        let mut agent = Agent::new(config, provider);

        if !agent_config.tools.is_empty() {
            let mut registry = ToolRegistry::new();
            for tool_name in &agent_config.tools {
                match tool_name.as_str() {
                    "bash" => registry.register(swiftforge_tools::BashTool::new()),
                    "read" => registry.register(swiftforge_tools::ReadTool::new()),
                    "write" => registry.register(swiftforge_tools::WriteTool::new()),
                    "edit" => registry.register(swiftforge_tools::EditTool::new()),
                    "grep" => registry.register(swiftforge_tools::GrepTool::new()),
                    _ => tracing::warn!("Unknown tool: {}", tool_name),
                }
            }
            agent = agent.with_tool_registry(Arc::new(registry));
        }

        self.agent = Some(agent);
        Ok(())
    }

    async fn execute_step(&mut self, step: &TestStep) -> Result<StepResult> {
        let start_time = Instant::now();
        let name = get_step_name(step).to_string();
        let action = get_step_action(step).to_string();

        let result = match step {
            TestStep::Chat { input, expect, .. } => {
                self.execute_chat(input, expect).await
            }
            TestStep::ToolCall { tool, arguments, expect, .. } => {
                self.execute_tool_call(tool, arguments, expect).await
            }
            TestStep::Assert { conditions, .. } => {
                self.execute_assert(conditions).await
            }
            TestStep::Wait { duration_ms, .. } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*duration_ms)).await;
                Ok(StepResult::success(&name, &action, start_time.elapsed().as_millis() as u64))
            }
            TestStep::Loop { .. } => {
                Ok(StepResult::success(&name, &action, start_time.elapsed().as_millis() as u64))
            }
            TestStep::Condition { condition, then, else_branch, .. } => {
                self.execute_condition(condition, then, else_branch.as_deref()).await
            }
            TestStep::Snapshot { snapshot_name, data, .. } => {
                self.execute_snapshot(snapshot_name, data).await
            }
        };

        result.map(|mut r| {
            r.name = name;
            r.action = action;
            r.duration_ms = start_time.elapsed().as_millis() as u64;
            r
        })
    }

    async fn execute_chat(
        &mut self,
        input: &crate::parser::ChatInput,
        expect: &crate::parser::ChatExpect,
    ) -> Result<StepResult> {
        let agent = self.agent.as_ref().ok_or_else(|| anyhow::anyhow!("Agent not initialized"))?;

        let messages: Vec<Message> = input.messages.iter().map(|m| Message {
            role: m.role.clone(),
            content: m.content.clone(),
        }).collect();

        for msg in &messages {
            self.context.add_message(&msg.role, &msg.content);
        }

        let response = agent.chat(messages.clone()).await
            .map_err(|e| anyhow::anyhow!("Chat failed: {}", e))?;

        self.context.set_response(response.clone());

        if let Err(e) = Verifier::verify_chat_expect(&response, expect) {
            return Ok(StepResult::failure("chat", "chat", e.to_string(), 0));
        }

        Ok(StepResult::success("chat", "chat", 0).with_response(response.content))
    }

    async fn execute_tool_call(
        &mut self,
        tool_name: &str,
        arguments: &std::collections::HashMap<String, serde_json::Value>,
        expect: &crate::parser::ToolExpect,
    ) -> Result<StepResult> {
        let agent = self.agent.as_ref().ok_or_else(|| anyhow::anyhow!("Agent not initialized"))?;

        let args_value = serde_json::to_value(arguments.clone())?;
        let result = agent.call_tool(tool_name, args_value).await
            .map_err(|e| anyhow::anyhow!("Tool call failed: {}", e))?;

        self.context.add_tool_result(result.clone());

        if let Err(e) = Verifier::verify_tool_expect(&result, expect) {
            return Ok(StepResult::failure("tool_call", "tool_call", e.to_string(), 0));
        }

        let output = result.output.unwrap_or_default();
        Ok(StepResult::success("tool_call", "tool_call", 0).with_tool_result(output))
    }

    async fn execute_assert(&mut self, conditions: &[String]) -> Result<StepResult> {
        let mut failed_conditions = Vec::new();

        for condition in conditions {
            if !Verifier::evaluate_condition(condition, &self.context) {
                failed_conditions.push(condition.clone());
            }
        }

        if failed_conditions.is_empty() {
            Ok(StepResult::success("assert", "assert", 0).with_conditions(conditions.to_vec()))
        } else {
            let msg = format!("Conditions failed: {}", failed_conditions.join(", "));
            Ok(StepResult::failure("assert", "assert", msg, 0).with_conditions(conditions.to_vec()))
        }
    }

    async fn execute_loop(&mut self, times: usize, steps: &[TestStep]) -> Result<StepResult> {
        for i in 0..times {
            tracing::debug!("Loop iteration {}/{}", i + 1, times);
            for step in steps {
                let result = self.execute_step(step).await?;
                if !result.success {
                    return Ok(result);
                }
            }
            self.context.reset_for_loop();
        }
        Ok(StepResult::success("loop", "loop", 0))
    }

    async fn execute_condition(
        &mut self,
        condition: &str,
        _then_steps: &[TestStep],
        _else_steps: Option<&[TestStep]>,
    ) -> Result<StepResult> {
        let result = Verifier::evaluate_condition(condition, &self.context);
        if result {
            Ok(StepResult::success("condition", "condition", 0))
        } else {
            Ok(StepResult::failure("condition", "condition", "Condition evaluated to false".to_string(), 0))
        }
    }

    async fn execute_snapshot(
        &mut self,
        snapshot_name: &str,
        data: &std::collections::HashMap<String, String>,
    ) -> Result<StepResult> {
        let mut snapshot_data = serde_json::Map::new();

        for (key, value_spec) in data {
            let value = match value_spec.as_str() {
                "response.content.length" => {
                    serde_json::Value::Number(self.context.get_last_response_content()
                        .map(|c| c.len() as u64).unwrap_or(0).into())
                }
                "len(tool_results)" => {
                    serde_json::Value::Number(self.context.get_tool_result_count().into())
                }
                _ => {
                    if let Some(response) = self.context.get_last_response_content() {
                        serde_json::Value::String(response)
                    } else {
                        serde_json::Value::Null
                    }
                }
            };
            snapshot_data.insert(key.clone(), value);
        }

        self.context.save_snapshot(snapshot_name, serde_json::Value::Object(snapshot_data));

        Ok(StepResult::success("snapshot", "snapshot", 0))
    }
}