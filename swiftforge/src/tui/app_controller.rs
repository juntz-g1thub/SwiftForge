use anyhow::Result;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{
    event::{self, Event},
    execute,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use swiftforge_log::{debug, info, trace, warn};
use tokio::runtime::Builder;
use tokio::sync::RwLock;

use crate::core::SessionManager;
use crate::tui::config::ConfigManager;
use swiftforge_mcp::{McpConnectionPool, McpToolLoader};
// Providers are initialized in the `initializer` module
use swiftforge_types::Session;

use crate::tui::initializer::initialize_components;
use crate::tui::task::coordinator::TaskCoordinator;
use crate::tui::task::events::{CoordinatorEvent, TaskType};
use crate::tui::{
    Action, AppContext, ChatView, ConfigView, StreamingState, UIState, View, ViewState,
};
use swiftforge_providers::{
    AnthropicProvider, DeepSeekProvider, MiniMaxProvider, OllamaProvider, OpenAIProvider,
};
use uuid::Uuid;

pub struct AppController {
    context: AppContext,
    ui_state: UIState,
    runtime: tokio::runtime::Runtime,
    current_view: Box<dyn View>,
    should_quit: bool,
    coordinator: TaskCoordinator,
    current_task_id: Option<Uuid>,
    #[allow(unused)]
    mcp_pool: Option<Arc<McpConnectionPool>>,
    #[allow(unused)]
    mcp_loader: Option<Arc<McpToolLoader>>,
    session_manager: Option<Arc<SessionManager>>,
}

impl AppController {
    pub fn new() -> Result<Self> {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        let config = ConfigManager::new();
        let session_manager = if let Some(data_dir) = config.get_session_data_dir() {
            match SessionManager::new(data_dir) {
                Ok(sm) => Some(Arc::new(sm)),
                Err(e) => {
                    warn!("[session]", "Failed to create SessionManager: {}", e);
                    None
                }
            }
        } else {
            None
        };

        if let Some(ref sm) = session_manager {
            if let Err(e) = runtime.block_on(sm.create_session("default", 128_000)) {
                warn!("[session]", "Failed to create default session: {}", e);
            }
        }

        let components = initialize_components(&runtime.handle())?;

        let context = AppContext::new(components.agent.clone(), config, Arc::clone(&components.tool_registry));
        let ui_state = UIState::new();

        let provider = components.provider_name.clone();
        let model = components.model_name.clone();
        let current_view: Box<dyn View> = Box::new(ChatView::new(&provider, &model));

        let coordinator = TaskCoordinator::new();

        Ok(Self {
            context,
            ui_state,
            runtime,
            current_view,
            should_quit: false,
            coordinator,
            current_task_id: None,
            mcp_pool: Some(components.mcp_pool),
            mcp_loader: Some(components.mcp_loader),
            session_manager,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        info!("[app_controller]", "Application started");

        loop {
            // Drive the current_thread runtime to make progress on spawned agent tasks
            self.runtime.block_on(async {
                tokio::task::yield_now().await;
            });

            self.process_agent_response();

            trace!("[app_controller]", "loop: drawing");
            terminal.draw(|f| {
                self.current_view
                    .render(f, f.size(), &self.context, &self.ui_state);
            })?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    trace!("[app_controller]", "loop: key event {:?}", key);
                    if let Some(action) = self.current_view.handle_key(key, &self.context) {
                        info!("[app_controller]", "Action: {:?}", action);
                        self.handle_action(action)?;

                        if self.should_quit {
                            break;
                        }
                    }
                }
            }
        }

        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;

        Ok(())
    }

    fn handle_action(&mut self, action: Action) -> Result<()> {
        info!("[app_controller]", "handle_action: {:?}", action);

        match action {
            Action::SendMessage(msg) => {
                if let Some(chat_view) = self.get_chat_view_mut() {
                    chat_view.state.add_message("user", &msg);
                    chat_view.state.streaming_state = StreamingState::Streaming;
                }
                self.spawn_agent_task(msg);
            }
            Action::SwitchView(view_state) => {
                self.current_view.on_exit();
                self.current_view = match view_state {
                    ViewState::Chat(ctx) => {
                        Box::new(ChatView::new(&ctx.current_provider, &ctx.current_model))
                    }
                    ViewState::Config(ctx) => {
                        let mut view = ConfigView::new();
                        view.state = ctx;
                        Box::new(view)
                    }
                };
                self.current_view.on_enter();
            }
            Action::GoBack => {
                self.current_view.on_exit();
                let provider = self
                    .context
                    .config
                    .lock()
                    .unwrap()
                    .get_provider()
                    .to_string();
                let model = self
                    .context
                    .config
                    .lock()
                    .unwrap()
                    .get_model(&provider)
                    .to_string();
                self.current_view = Box::new(ChatView::new(&provider, &model));
                self.current_view.on_enter();
            }
            Action::CancelStreaming => {
                self.ui_state.clear_streaming();
                if let Some(chat_view) = self.get_chat_view_mut() {
                    chat_view.state.streaming_state = StreamingState::Idle;
                }
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::SelectProvider(name) => {
                self.context.config.lock().unwrap().set_provider(&name);
            }
            Action::SaveApiKey(key) => {
                let provider = self
                    .context
                    .config
                    .lock()
                    .unwrap()
                    .get_provider()
                    .to_string();
                self.context
                    .config
                    .lock()
                    .unwrap()
                    .set_api_key(&provider, Some(key));
            }
            Action::SaveModel(model) => {
                let provider = self
                    .context
                    .config
                    .lock()
                    .unwrap()
                    .get_provider()
                    .to_string();
                self.context
                    .config
                    .lock()
                    .unwrap()
                    .set_model(&provider, model);
            }
            Action::SaveBaseUrl(url) => {
                let provider = self
                    .context
                    .config
                    .lock()
                    .unwrap()
                    .get_provider()
                    .to_string();
                self.context
                    .config
                    .lock()
                    .unwrap()
                    .set_base_url(&provider, Some(url));
            }
            Action::FetchModels => {
                self.spawn_fetch_models();
            }
            _ => {}
        }

        Ok(())
    }

    fn get_chat_view_mut(&mut self) -> Option<&mut ChatView> {
        let any_ref: &mut dyn std::any::Any = &mut *self.current_view;
        any_ref.downcast_mut::<ChatView>()
    }

    fn spawn_agent_task(&mut self, msg: String) {
        let task_id = self.coordinator.enqueue(TaskType::UserMessage { priority: 100 });
        self.current_task_id = Some(task_id);
        debug!("[app_controller]", "spawn_agent_task START, msg={}", msg);
        trace!(
            "[app_controller]",
            "SPAWN: spawn_agent_task called, msg_len={}",
            msg.len()
        );
        let runtime = self.runtime.handle().clone();

        let (tx, rx) = mpsc::channel();
        debug!("[app_controller]", "Created channel, storing rx");
        *self.ui_state.response_receiver.lock().unwrap() = Some(rx);

        let provider_name = self
            .context
            .config
            .lock()
            .unwrap()
            .get_provider()
            .to_string();
        let _api_key = self
            .context
            .config
            .lock()
            .unwrap()
            .get_api_key(&provider_name);
        let _base_url = self
            .context
            .config
            .lock()
            .unwrap()
            .get_base_url(&provider_name);
        let _model = self
            .context
            .config
            .lock()
            .unwrap()
            .get_model(&provider_name)
            .to_string();
        let agent = self.context.agent.clone();

        let streaming_text = Arc::clone(&self.ui_state.streaming_text);
        let finalized_message = Arc::clone(&self.ui_state.finalized_message);
        let session_manager = self.session_manager.clone();

        runtime.spawn(async move {
            debug!(
                "[app_controller]",
                "SPAWN: task started, provider={}", provider_name
            );

            let final_agent = agent;

            {
                if let Ok(mut streaming) = streaming_text.lock() {
                    *streaming = None;
                }
            }

            let (agent_tx, agent_rx) = mpsc::channel::<Result<String>>();
            let streaming_text_clone = Arc::clone(&streaming_text);
            let tx_for_ui = tx.clone();

            std::thread::spawn(move || {
                debug!("[app_controller]", "thread: Background thread started");
                while let Ok(result) = agent_rx.recv() {
                    debug!("[app_controller]", "thread: Received chunk from agent");
                    if let Ok(text) = &result {
                        debug!("[app_controller]", "thread: Chunk text: {}", text.chars().take(50).collect::<String>());
                        if let Ok(mut s) = streaming_text_clone.lock() {
                            if let Some(ref mut content) = *s {
                                content.push_str(text);
                            } else {
                                *s = Some(text.clone());
                            }
                        }
                    }
                    let _ = tx_for_ui.send(result);
                    debug!("[app_controller]", "thread: Forwarded to UI");
                }
                debug!("[app_controller]", "thread: Channel closed, thread ending");
            });

            debug!("[app_controller]", "About to create session");
            let session = if let Some(ref sm) = session_manager {
                sm.get_current_session()
                    .await
                    .map(|s| Arc::new(RwLock::new(s)))
                    .unwrap_or_else(|| {
                        debug!("[app_controller]", "No current session in manager, creating temp");
                        Arc::new(RwLock::new(Session::new(
                            uuid::Uuid::new_v4().to_string(),
                            "temp".to_string(),
                            100,
                        )))
                    })
            } else {
                debug!("[app_controller]", "No session_manager, creating temp session");
                Arc::new(RwLock::new(Session::new(
                    uuid::Uuid::new_v4().to_string(),
                    "temp".to_string(),
                    100,
                )))
            };
            debug!("[app_controller]", "Session created, calling run_agent_loop");

            let result = final_agent.run_agent_loop(session, &msg, 5, Some(agent_tx)).await;
            debug!("[app_controller]", "run_agent_loop returned, result={:?}", result.is_ok());

            match result {
                Ok(response) => {
                    if !response.is_empty() {
                        if let Ok(mut finalized) = finalized_message.lock() {
                            *finalized = Some(("assistant".to_string(), response));
                        }
                    }
                }
                Err(e) => {
                    let partial = streaming_text
                        .lock()
                        .map(|mut s| s.take().unwrap_or_default())
                        .unwrap_or_default();
                    if let Ok(mut finalized) = finalized_message.lock() {
                        *finalized =
                            Some(("error".to_string(), format!("{} (partial: {})", e, partial)));
                    }
                }
            }
        });
    }

    fn spawn_fetch_models(&mut self) {
        let runtime = self.runtime.handle().clone();

        let provider_name = self
            .context
            .config
            .lock()
            .unwrap()
            .get_provider()
            .to_string();
        let api_key = self
            .context
            .config
            .lock()
            .unwrap()
            .get_api_key(&provider_name);
        let base_url = self
            .context
            .config
            .lock()
            .unwrap()
            .get_base_url(&provider_name);
        let model = self
            .context
            .config
            .lock()
            .unwrap()
            .get_model(&provider_name)
            .to_string();
        let config = Arc::clone(&self.context.config);

        runtime.spawn(async move {
            let model_opt = Some(model.clone());

            let models = match provider_name.as_str() {
                "openai" => {
                    let p = OpenAIProvider::new(api_key.unwrap_or_default(), base_url);
                    p.list_models().await.unwrap_or_default()
                }
                "anthropic" => {
                    let p = AnthropicProvider::new(api_key.unwrap_or_default(), base_url);
                    p.list_models().await.unwrap_or_default()
                }
                "ollama" => {
                    let p = OllamaProvider::new(base_url, model_opt.clone());
                    p.list_models().await.unwrap_or_default()
                }
                "deepseek" => {
                    let p = DeepSeekProvider::new(
                        api_key.unwrap_or_default(),
                        base_url,
                        model_opt.clone(),
                    );
                    p.list_models().await.unwrap_or_default()
                }
                "minimax" => {
                    let p = MiniMaxProvider::new(
                        api_key.unwrap_or_default(),
                        base_url,
                        model_opt.clone(),
                    );
                    p.list_models().await.unwrap_or_default()
                }
                _ => Vec::new(),
            };

            for m in models {
                config.lock().unwrap().set_model(&provider_name, m);
            }
        });
    }

    fn process_agent_response(&mut self) {
        debug!("[app_controller]", "process_agent_response called");

        let finalized_msg = {
            if let Ok(mut finalized) = self.ui_state.finalized_message.lock() {
                finalized.take()
            } else {
                None
            }
        };

        let mut channel_disconnected = false;
        {
            if let Ok(receiver) = self.ui_state.response_receiver.lock() {
                if let Some(ref rx) = *receiver {
                    debug!("[app_controller]", "Polling rx.try_recv()");
                    loop {
                        match rx.try_recv() {
                            Ok(Ok(chunk)) => {
                                debug!(
                                    "[app_controller]",
                                    "CHUNK: received, chunk_len={} (forwarded to streaming_text)",
                                    chunk.len()
                                );
                            }
                            Ok(Err(e)) => {
                                debug!("[app_controller]", "CHUNK: agent error: {:?}", e);
                            }
                            Err(mpsc::TryRecvError::Empty) => {
                                break;
                            }
                            Err(mpsc::TryRecvError::Disconnected) => {
                                debug!("[app_controller]", "CHANNEL: disconnected");
                                channel_disconnected = true;
                                break;
                            }
                        }
                    }
                } else {
                    debug!("[app_controller]", "response_receiver is None");
                }
            }
        }

        // Extract data from self BEFORE borrowing chat_view (avoids borrow checker conflict)
        let is_streaming = self
            .get_chat_view_mut()
            .map(|cv| cv.state.streaming_state.is_active())
            .unwrap_or(false);

        // Clean up shared state based on event type
        let streaming_fallback = if finalized_msg.is_some() {
            self.ui_state.clear_streaming();
            if let Ok(mut receiver) = self.ui_state.response_receiver.lock() {
                *receiver = None;
            }
            None
        } else if channel_disconnected && is_streaming {
            let text = self
                .ui_state
                .streaming_text
                .lock()
                .ok()
                .and_then(|mut s| s.take());
            if let Ok(mut receiver) = self.ui_state.response_receiver.lock() {
                *receiver = None;
            }
            text
        } else {
            None
        };

        // Apply state transitions to chat_view (single borrow)
        if let Some(chat_view) = self.get_chat_view_mut() {
            if let Some((ref role, ref content)) = finalized_msg {
                debug!(
                    "[app_controller]",
                    "FINALIZED: adding to messages, role={}, content_len={}",
                    role,
                    content.len()
                );
                    chat_view.state.add_message(&role, content);
                chat_view.state.streaming_state = StreamingState::Completed;
            } else if let Some(text) = streaming_fallback {
                if !text.is_empty() {
                    debug!(
                        "[app_controller]",
                        "DISCONNECTED: migrating streaming_text to messages, len={}",
                        text.len()
                    );
                    chat_view.state.add_message("assistant", &text);
                    chat_view.state.streaming_state = StreamingState::Completed;
                }
            }
        }

        // Notify coordinator on task completion
        if finalized_msg.is_some() || channel_disconnected {
            if let Some(task_id) = self.current_task_id.take() {
                self.coordinator.process_event(CoordinatorEvent::TaskCompleted { task_id });
            }
        }
    }
}
