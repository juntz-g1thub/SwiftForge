use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Constraint, Direction, Layout, Margin};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use std::io::Stdout;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

use crate::providers::{LLMProvider, OpenAIProvider, AnthropicProvider, OllamaProvider, DeepSeekProvider, MiniMaxProvider, CustomProvider};
use crate::core::Message;
use crate::tui::config::ConfigManager;

fn display_width(c: char) -> usize {
    match c {
        c if c.is_ascii() => 1,
        c if c == '\t' => 4,
        c if c >= '\u{3000}' && c <= '\u{303F}' => 2,
        c if c >= '\u{3040}' && c <= '\u{309F}' => 2,
        c if c >= '\u{30A0}' && c <= '\u{30FF}' => 2,
        c if c >= '\u{3100}' && c <= '\u{312F}' => 2,
        c if c >= '\u{3130}' && c <= '\u{318F}' => 2,
        c if c >= '\u{3190}' && c <= '\u{319F}' => 2,
        c if c >= '\u{31A0}' && c <= '\u{31BF}' => 2,
        c if c >= '\u{31C0}' && c <= '\u{31EF}' => 2,
        c if c >= '\u{31F0}' && c <= '\u{31FF}' => 2,
        c if c >= '\u{3200}' && c <= '\u{32FF}' => 2,
        c if c >= '\u{3300}' && c <= '\u{33FF}' => 2,
        c if c >= '\u{FE30}' && c <= '\u{FE4F}' => 2,
        c if c >= '\u{FE50}' && c <= '\u{FE6F}' => 2,
        c if c >= '\u{FF00}' && c <= '\u{FFEF}' => 2,
        c if c >= '\u{2E80}' && c <= '\u{A4CF}' => 2,
        c if c >= '\u{AC00}' && c <= '\u{D7AF}' => 2,
        _ => 1,
    }
}

fn text_width(text: &str) -> usize {
    text.chars().map(display_width).sum()
}

fn wrap_text_to_width(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    for ch in text.chars() {
        let ch_width = display_width(ch);
        if current_width + ch_width > width {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
                current_width = 0;
            }
        }
        current.push(ch);
        current_width += ch_width;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn pad_to_display_width(text: &str, target_width: usize) -> String {
    let display_len = text_width(text);
    if display_len >= target_width {
        return text.to_string();
    }
    let remaining = target_width - display_len;
    format!("{}{}", text, " ".repeat(remaining))
}

fn render_ascii_table(headers: &[String], rows: &[Vec<String>]) -> Vec<Line<'static>> {
    let mut result: Vec<Line<'static>> = Vec::new();

    if headers.is_empty() && rows.is_empty() {
        return result;
    }

    let max_cols = headers.len().max(rows.first().map(|r| r.len()).unwrap_or(0));
    if max_cols == 0 {
        return result;
    }

    let terminal_width = 80;
    let min_col_width = 6;

    let mut col_widths = vec![min_col_width; max_cols];

    for (i, h) in headers.iter().enumerate() {
        col_widths[i] = col_widths[i].max(text_width(h));
    }

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < max_cols {
                col_widths[i] = col_widths[i].max(text_width(cell));
            }
        }
    }

    let total: usize = col_widths.iter().sum();
    let total_with_separators = total + max_cols + 1;
    if total_with_separators > terminal_width {
        let available = terminal_width - max_cols - 1;
        let scale = available as f64 / total as f64;
        col_widths = col_widths.iter().map(|w| (*w as f64 * scale).ceil() as usize).collect();
        for col in col_widths.iter_mut() {
            if *col < 2 {
                *col = 2;
            }
        }
    }

    let top_border = {
        let parts: Vec<String> = col_widths.iter().map(|w| "─".repeat(*w)).collect();
        format!("┌{}┐", parts.join("┬"))
    };

    let middle_border = {
        let parts: Vec<String> = col_widths.iter().map(|w| "─".repeat(*w)).collect();
        format!("├{}┤", parts.join("┼"))
    };

    let bottom_border = {
        let parts: Vec<String> = col_widths.iter().map(|w| "─".repeat(*w)).collect();
        format!("└{}┘", parts.join("┴"))
    };

    let mut total_lines: Vec<Line<'static>> = Vec::new();

    if !headers.is_empty() {
        total_lines.push(Line::from(Span::raw(top_border.clone())));
        let header_cells: Vec<String> = headers.iter().enumerate().map(|(i, s)| {
            pad_to_display_width(s, col_widths[i])
        }).collect();
        total_lines.push(Line::from(Span::raw(format!("│{}│", header_cells.join("│")))));
        total_lines.push(Line::from(Span::raw(middle_border.clone())));
    }

    let row_count = rows.len();
    for (idx, row) in rows.iter().enumerate() {
        let is_last_row = idx == row_count.saturating_sub(1);
        let row_cells: Vec<String> = row.iter().take(max_cols).cloned().collect();

        let cell_lines: Vec<Vec<String>> = row_cells.iter().enumerate().map(|(i, cell)| {
            wrap_text_to_width(cell, col_widths[i])
        }).collect();

        let max_lines_in_row = cell_lines.iter().map(|l| l.len()).max().unwrap_or(1);

        for line_idx in 0..max_lines_in_row {
            let line_cells: Vec<String> = cell_lines.iter().enumerate().map(|(i, lines)| {
                let line_text = lines.get(line_idx).map(|s| s.as_str()).unwrap_or("");
                pad_to_display_width(line_text, col_widths[i])
            }).collect();

            let is_last_line_of_last_row = is_last_row && line_idx == max_lines_in_row - 1;
            let left = if is_last_line_of_last_row { "└" } else { "│" };
            let right = if is_last_line_of_last_row { "┘" } else { "│" };

            total_lines.push(Line::from(Span::raw(format!("{}{}{}", left, line_cells.join("│"), right))));
        }

        if !is_last_row {
            total_lines.push(Line::from(Span::raw(middle_border.clone())));
        }
    }

    if !rows.is_empty() {
        total_lines.push(Line::from(Span::raw(bottom_border)));
    }

    result = total_lines;
    result
}

fn render_markdown(content: &str) -> Vec<Line<'_>> {
    use pulldown_cmark::{Parser, Event, Tag, TagEnd, Options};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(content, options);
    let mut lines: Vec<Line<'_>> = Vec::new();
    let mut current_spans: Vec<Span<'_>> = Vec::new();
    let mut in_code_block = false;
    let mut code_block_style = Style::default();
    let mut in_heading = false;
    let mut heading_color = Color::Cyan;

    let mut table_data: Vec<Vec<String>> = Vec::new();
    let mut table_headers: Vec<String> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    let mut in_table = false;
    let mut in_table_head = false;

    fn push_line<'a>(lines: &mut Vec<Line<'a>>, current_spans: &mut Vec<Span<'a>>) {
        if !current_spans.is_empty() {
            lines.push(Line::from(std::mem::take(current_spans)));
        }
    }

    for event in parser {
        match event {
            Event::Start(tag) => {
                match tag {
                    Tag::Heading { level: ref lvl_ref, .. } => {
                        push_line(&mut lines, &mut current_spans);
                        in_heading = true;
                        let lvl_val = *lvl_ref as usize;
                        heading_color = match lvl_val {
                            1 => Color::Cyan,
                            2 => Color::Magenta,
                            3 => Color::Yellow,
                            _ => Color::Green,
                        };
                        current_spans.push(Span::raw(" ".repeat(lvl_val)));
                    }
                    Tag::Strong => {
                        current_spans.push(Span::styled(String::new(), Style::new().bold()));
                    }
                    Tag::Emphasis => {
                        current_spans.push(Span::styled(String::new(), Style::new().italic()));
                    }
                    Tag::CodeBlock(kind) => {
                        push_line(&mut lines, &mut current_spans);
                        in_code_block = true;
                        code_block_style = match kind {
                            pulldown_cmark::CodeBlockKind::Fenced(_) => {
                                Style::default().bg(Color::Black).fg(Color::White)
                            }
                            pulldown_cmark::CodeBlockKind::Indented => {
                                Style::default().bg(Color::Black).fg(Color::White)
                            }
                        };
                    }
                    Tag::BlockQuote => {
                        current_spans.push(Span::styled(String::from("│ "), Style::new().dim().fg(Color::Cyan)));
                    }
                    Tag::List(_) => {}
                    Tag::Item => {
                        push_line(&mut lines, &mut current_spans);
                        current_spans.push(Span::styled(String::from("• "), Style::new().fg(Color::Yellow)));
                    }
                    Tag::Table(_) => {
                        in_table = true;
                        table_data.clear();
                        table_headers.clear();
                    }
                    Tag::TableHead => {
                        in_table_head = true;
                        table_headers.clear();
                    }
                    Tag::TableRow => {
                        current_row.clear();
                    }
                    Tag::TableCell => {
                        current_cell.clear();
                    }
                    _ => {}
                }
            }
            Event::End(tag) => {
                match tag {
                    TagEnd::Heading(_) => {
                        in_heading = false;
                        push_line(&mut lines, &mut current_spans);
                    }
                    TagEnd::Strong => {}
                    TagEnd::Emphasis => {}
                    TagEnd::CodeBlock => {
                        in_code_block = false;
                        push_line(&mut lines, &mut current_spans);
                    }
                    TagEnd::Item => {
                        push_line(&mut lines, &mut current_spans);
                    }
                    TagEnd::Paragraph => {
                        push_line(&mut lines, &mut current_spans);
                    }
                    TagEnd::TableCell => {
                        current_row.push(current_cell.clone());
                    }
                    TagEnd::TableRow => {
                        if in_table_head {
                            table_headers = current_row.clone();
                        } else {
                            table_data.push(current_row.clone());
                        }
                    }
                    TagEnd::TableHead => {
                        in_table_head = false;
                    }
                    TagEnd::Table => {
                        in_table = false;
                        let headers_copy = table_headers.clone();
                        let data_copy = table_data.clone();
                        let table_lines = render_ascii_table(&headers_copy, &data_copy);
                        lines.extend(table_lines);
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                if in_table {
                    current_cell.push_str(&text);
                } else if text.starts_with("[thinking]") {
                    current_spans.push(Span::styled("[thinking] ", Style::new().bold().fg(Color::DarkGray).on_dark_gray()));
                    let rest = text.replace("[thinking]", "");
                    if !rest.is_empty() {
                        current_spans.push(Span::styled(rest, Style::new().fg(Color::DarkGray).on_dark_gray()));
                    }
                } else if in_code_block {
                    for (i, line) in text.split('\n').enumerate() {
                        if i > 0 {
                            push_line(&mut lines, &mut current_spans);
                        }
                        current_spans.push(Span::styled(line.to_string(), code_block_style));
                    }
                } else if in_heading {
                    for (i, line) in text.split('\n').enumerate() {
                        if i > 0 {
                            push_line(&mut lines, &mut current_spans);
                        }
                        current_spans.push(Span::styled(line.to_string(), Style::new().bold().fg(heading_color)));
                    }
                } else {
                    for (i, line) in text.split('\n').enumerate() {
                        if i > 0 {
                            push_line(&mut lines, &mut current_spans);
                        }
                        current_spans.push(Span::raw(line.to_string()));
                    }
                }
            }
            Event::Code(code) => {
                current_spans.push(Span::styled(format!("`{}`", code), Style::new().fg(Color::Yellow)));
            }
            Event::SoftBreak => {
                current_spans.push(Span::raw(" "));
            }
            Event::HardBreak => {
                push_line(&mut lines, &mut current_spans);
            }
            _ => {}
        }
    }

    push_line(&mut lines, &mut current_spans);

    if lines.is_empty() {
        lines.push(Line::from(Span::raw(content)));
    }

    lines
}

pub struct App {
    config: ConfigManager,
    mode: AppMode,
    messages: Vec<(String, String)>,
    input: String,
    cursor_char_pos: usize,
    should_quit: bool,
    streaming_text: Option<String>,
    response_receiver: Option<mpsc::Receiver<Result<String>>>,
    fetched_models: Vec<String>,
    model_fetch_error: Option<String>,
    model_fetch_receiver: Option<mpsc::Receiver<Result<Vec<String>>>>,
    system_prompt: Option<String>,
    scrollbar_state: ScrollbarState,
    scroll_offset: usize,
    content_height: usize,
}

pub enum AppMode {
    Chat,
    ConfigProvider,
    ConfigApiKey,
    ConfigModel,
    ConfigUrl,
    ConfigCustomName,
    ConfigCustomUrl,
    ConfigFetchModels,
    ConfigSelectModel,
}

impl App {
    pub fn new() -> Self {
        let config = ConfigManager::new();
        let system_prompt = config.get_system_prompt();
        Self {
            config,
            mode: AppMode::Chat,
            messages: Vec::new(),
            input: String::new(),
            cursor_char_pos: 0,
            should_quit: false,
            streaming_text: None,
            response_receiver: None,
            fetched_models: Vec::new(),
            model_fetch_error: None,
            model_fetch_receiver: None,
            system_prompt,
            scrollbar_state: ScrollbarState::new(0),
            scroll_offset: 0,
            content_height: 0,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key_event(key)?;
                    if self.should_quit {
                        break;
                    }
                }
            }

            if let Some(ref receiver) = self.response_receiver {
                match receiver.try_recv() {
                    Ok(Ok(chunk)) => {
                        if let Some(ref mut streaming) = self.streaming_text {
                            streaming.push_str(&chunk);
                        } else {
                            self.streaming_text = Some(chunk);
                        }
                    }
                    Ok(Err(e)) => {
                        let partial = self.streaming_text.clone().unwrap_or_default();
                        self.messages.push(("error".to_string(), format!("{} (partial: {})", e, partial)));
                        self.streaming_text = None;
                        self.response_receiver = None;
                    }
                    Err(TryRecvError::Disconnected) => {
                        if let Some(final_text) = self.streaming_text.take() {
                            if !final_text.is_empty() {
                                self.messages.push(("assistant".to_string(), final_text));
                            }
                        }
                        self.response_receiver = None;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            }

            if let Some(ref receiver) = self.model_fetch_receiver {
                if let Ok(result) = receiver.try_recv() {
                    match result {
                        Ok(models) => {
                            if models.is_empty() {
                                self.model_fetch_error = Some("No models found".to_string());
                            } else {
                                self.fetched_models = models;
                                self.mode = AppMode::ConfigSelectModel;
                            }
                        }
                        Err(e) => {
                            self.model_fetch_error = Some(e.to_string());
                        }
                    }
                    self.model_fetch_receiver = None;
                }
            }
        }
        Ok(())
    }

    fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
        let mut char_count = 0;
        for (byte_idx, _c) in s.char_indices() {
            if char_count >= char_idx {
                return byte_idx;
            }
            char_count += 1;
        }
        s.len()
    }

    fn remove_char_before_cursor(&mut self) {
        if self.cursor_char_pos == 0 || self.input.is_empty() {
            return;
        }
        let target_char_pos = self.cursor_char_pos - 1;
        let byte_idx = Self::char_to_byte_idx(&self.input, target_char_pos);
        self.input.remove(byte_idx);
        self.cursor_char_pos = target_char_pos;
    }

    fn remove_char_at_cursor(&mut self) {
        let char_count = self.input.chars().count();
        if self.cursor_char_pos >= char_count || self.input.is_empty() {
            return;
        }
        let byte_idx = Self::char_to_byte_idx(&self.input, self.cursor_char_pos);
        self.input.remove(byte_idx);
    }

    fn display_input_with_cursor(input: &str, char_pos: usize) -> String {
        if input.is_empty() {
            return String::from("> ▌");
        }
        let byte_pos = Self::char_to_byte_idx(input, char_pos.min(input.chars().count()));
        let before = &input[..byte_pos];
        let after = &input[byte_pos..];
        format!("> {}{}▌", before, after)
    }

    fn render(&mut self, f: &mut Frame) {
        match self.mode {
            AppMode::Chat => self.render_chat(f),
            AppMode::ConfigProvider => self.render_config_provider(f),
            AppMode::ConfigApiKey => self.render_config_input(f, "API Key", "Enter API key..."),
            AppMode::ConfigModel => self.render_config_input(f, "Model", "Enter model name..."),
            AppMode::ConfigUrl => self.render_config_input(f, "Base URL", "Enter base URL..."),
            AppMode::ConfigCustomName => self.render_config_input(f, "Custom Provider Name", "Enter provider name..."),
            AppMode::ConfigCustomUrl => self.render_config_input(f, "Custom Base URL", "Enter base URL..."),
            AppMode::ConfigFetchModels => self.render_fetching_models(f),
            AppMode::ConfigSelectModel => self.render_select_model(f),
        }
    }

    fn render_chat(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ].as_ref())
            .split(f.size());

        let current_provider = self.config.get_provider();
        let current_model = self.config.get_model(current_provider);

        let mut lines: Vec<Line> = Vec::new();
        for (role, content) in &self.messages {
            let role_style = match role.as_str() {
                "user" => Style::new().green().bold(),
                "assistant" => Style::new().cyan().bold(),
                "system" => Style::new().yellow().bold(),
                "error" => Style::new().red().bold(),
                _ => Style::new().white(),
            };
            let role_display = if role == "assistant" {
                format!("[{} {}]", role, current_model)
            } else {
                format!("[{}]", role)
            };
            let role_prefix = Span::styled(format!("{}: ", role_display), role_style);
            let content_lines = render_markdown(content);

            lines.push(Line::from(role_prefix));
            for content_line in content_lines {
                lines.push(Line::from(content_line.spans.iter().cloned().collect::<Vec<_>>()));
            }
        }
        if let Some(ref streaming) = self.streaming_text {
            let streaming_lines = render_markdown(streaming);
            let role_prefix = Span::styled(format!("[{} {}]: ", "assistant", current_model), Style::new().cyan().bold());
            lines.push(Line::from(role_prefix));
            for streaming_line in streaming_lines {
                lines.push(Line::from(streaming_line.spans.iter().cloned().collect::<Vec<_>>()));
            }
            lines.push(Line::from(Span::styled("▌", Style::new().slow_blink())));
        }

        self.content_height = lines.len();
        let visible_height = chunks[0].height as usize;

        let max_scroll = if self.content_height > visible_height {
            self.content_height - visible_height
        } else {
            0
        };
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
        self.scrollbar_state = ScrollbarState::new(max_scroll).position(self.scroll_offset);

        let scrollable_lines: Vec<Line> = if self.scroll_offset >= lines.len() {
            lines.clone()
        } else {
            lines[self.scroll_offset..].to_vec()
        };

        let paragraph = Paragraph::new(Text::from(scrollable_lines))
            .block(Block::default().borders(Borders::ALL).title("Chat (Ctrl+S: Settings)"))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, chunks[0]);

        let scrollbar_area = chunks[0].inner(&Margin { vertical: 1, horizontal: 0 });
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            scrollbar_area,
            &mut self.scrollbar_state,
        );

        let display_input = if self.input.is_empty() {
            String::from("> ")
        } else {
            Self::display_input_with_cursor(&self.input, self.cursor_char_pos)
        };

        let input_para = Paragraph::new(display_input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Input"));
        f.render_widget(input_para, chunks[1]);

        let char_offset: u16 = self.input.chars().take(self.cursor_char_pos).map(display_width).sum::<usize>() as u16;
        let cursor_x = chunks[1].x + 1 + 2 + char_offset;
        let cursor_y = chunks[1].y + 1;
        f.set_cursor(cursor_x, cursor_y);

        let status_provider = self.config.get_provider();
        let status_text = format!("[{}] - Press Ctrl+Q to quit, Ctrl+S for settings", status_provider);
        let status_para = Paragraph::new(status_text.as_str());
        f.render_widget(status_para, chunks[2]);
    }

    fn handle_key_event(&mut self, key: event::KeyEvent) -> Result<()> {
        match self.mode {
            AppMode::Chat => self.handle_chat_key(key),
            AppMode::ConfigProvider => self.handle_config_provider_key(key),
            AppMode::ConfigApiKey | AppMode::ConfigModel | AppMode::ConfigUrl |
            AppMode::ConfigCustomName | AppMode::ConfigCustomUrl => self.handle_config_input_key(key),
            AppMode::ConfigFetchModels => self.handle_fetch_models_key(key),
            AppMode::ConfigSelectModel => self.handle_select_model_key(key),
        }
    }

    fn insert_char_at_cursor(&mut self, c: char) {
        let byte_idx = Self::char_to_byte_idx(&self.input, self.cursor_char_pos);
        self.input.insert(byte_idx, c);
        self.cursor_char_pos += 1;
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_char_pos > 0 {
            self.cursor_char_pos -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_char_pos < self.input.chars().count() {
            self.cursor_char_pos += 1;
        }
    }

    fn move_cursor_to_start(&mut self) {
        self.cursor_char_pos = 0;
    }

    fn move_cursor_to_end(&mut self) {
        self.cursor_char_pos = self.input.chars().count();
    }

    fn handle_chat_key(&mut self, key: event::KeyEvent) -> Result<()> {
        if self.response_receiver.is_some() || self.streaming_text.is_some() {
            match key.code {
                KeyCode::Up => {
                    if self.scroll_offset > 0 {
                        self.scroll_offset -= 1;
                    }
                    return Ok(());
                }
                KeyCode::Down => {
                    let max_scroll = if self.content_height > 0 {
                        self.content_height.saturating_sub(1)
                    } else {
                        0
                    };
                    if self.scroll_offset < max_scroll {
                        self.scroll_offset += 1;
                    }
                    return Ok(());
                }
                KeyCode::Esc => {
                    self.streaming_text = None;
                    self.response_receiver = None;
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Char(c) if key.modifiers == event::KeyModifiers::CONTROL => {
                match c {
                    'q' | 'Q' => {
                        self.should_quit = true;
                    }
                    's' | 'S' => {
                        self.mode = AppMode::ConfigProvider;
                    }
                    _ => {}
                }
            }
            KeyCode::Char(c) => {
                self.insert_char_at_cursor(c);
            }
            KeyCode::Backspace => {
                self.remove_char_before_cursor();
            }
            KeyCode::Delete => {
                self.remove_char_at_cursor();
            }
            KeyCode::Left => {
                self.move_cursor_left();
            }
            KeyCode::Right => {
                self.move_cursor_right();
            }
            KeyCode::Home => {
                self.move_cursor_to_start();
            }
            KeyCode::End => {
                self.move_cursor_to_end();
            }
            KeyCode::Enter => {
                if !self.input.trim().is_empty() {
                    let user_input = self.input.trim().to_string();
                    self.messages.push(("user".to_string(), user_input.clone()));
                    self.input.clear();
                    self.cursor_char_pos = 0;
                    self.streaming_text = Some(String::new());
                    self.send_to_provider();
                }
            }
            KeyCode::Esc => {}
            _ => {}
        }
        Ok(())
    }

    fn send_to_provider(&mut self) {
        let provider_name = self.config.get_provider().to_string();
        let api_key_opt = self.config.get_api_key(&provider_name);
        let base_url_opt = self.config.get_base_url(&provider_name);
        let model_opt = Some(self.config.get_model(&provider_name));

        let messages: Vec<Message> = self.messages.iter()
            .map(|(role, content)| Message { role: role.clone(), content: content.clone() })
            .collect();

        let (tx, rx) = mpsc::channel();
        self.response_receiver = Some(rx);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result: Result<(), anyhow::Error> = match provider_name.as_str() {
                    "openai" => {
                        let p = OpenAIProvider::new(api_key_opt.unwrap_or_default(), base_url_opt);
                        let tx_clone = tx.clone();
                        p.stream_chat(messages, move |chunk| {
                            let _ = tx_clone.send(Ok(chunk));
                        }).await
                    }
                    "anthropic" => {
                        let p = AnthropicProvider::new(api_key_opt.unwrap_or_default(), base_url_opt);
                        let tx_clone = tx.clone();
                        p.stream_chat(messages, move |chunk| {
                            let _ = tx_clone.send(Ok(chunk));
                        }).await
                    }
                    "ollama" => {
                        let p = OllamaProvider::new(base_url_opt, model_opt);
                        let tx_clone = tx.clone();
                        p.stream_chat(messages, move |chunk| {
                            let _ = tx_clone.send(Ok(chunk));
                        }).await
                    }
                    "deepseek" => {
                        let p = DeepSeekProvider::new(api_key_opt.unwrap_or_default(), base_url_opt, model_opt);
                        let tx_clone = tx.clone();
                        p.stream_chat(messages, move |chunk| {
                            let _ = tx_clone.send(Ok(chunk));
                        }).await
                    }
                    "minimax" => {
                        let p = MiniMaxProvider::new(api_key_opt.unwrap_or_default(), base_url_opt, model_opt);
                        let tx_clone = tx.clone();
                        p.stream_chat(messages, move |chunk| {
                            let _ = tx_clone.send(Ok(chunk));
                        }).await
                    }
                    "custom" => {
                        let name = "custom".to_string();
                        let p = CustomProvider::new(name, api_key_opt.unwrap_or_default(), base_url_opt.unwrap_or_default(), model_opt.unwrap_or_default());
                        let tx_clone = tx.clone();
                        p.stream_chat(messages, move |chunk| {
                            let _ = tx_clone.send(Ok(chunk));
                        }).await
                    }
                    _ => {
                        let p = OpenAIProvider::new(api_key_opt.unwrap_or_default(), base_url_opt);
                        let tx_clone = tx.clone();
                        p.stream_chat(messages, move |chunk| {
                            let _ = tx_clone.send(Ok(chunk));
                        }).await
                    }
                };

                if let Err(e) = result {
                    let _ = tx.send(Err(e));
                }
            });
        });
    }

    fn render_config_provider(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ].as_ref())
            .split(f.size());

        let current_provider = self.config.get_provider();

        f.render_widget(
            Paragraph::new(format!("Current: {}", current_provider))
                .block(Block::default().borders(Borders::ALL).title("Provider")),
            chunks[0]
        );

        let mut items = vec![];

        let providers = [
            ("openai", "OpenAI"),
            ("anthropic", "Anthropic"),
            ("ollama", "Ollama"),
            ("deepseek", "DeepSeek"),
            ("minimax", "MiniMax"),
            ("custom", "Custom"),
        ];

        for (id, name) in providers {
            let marker = if id == current_provider { ">" } else { " " };
            items.push(format!("{} [{}]", marker, name));
        }

        let items: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
        let list = Paragraph::new(Text::from(items.join("\n")))
            .block(Block::default().borders(Borders::ALL).title("Select Provider"));

        f.render_widget(list, chunks[1]);

        let custom_count = 0;
        f.render_widget(
            Paragraph::new(format!("[C] Custom Provider ({} configured)", custom_count))
                .block(Block::default().borders(Borders::ALL).title("Custom")),
            chunks[2]
        );

        f.render_widget(
            Paragraph::new("[F] Fetch Models | [ESC] Back to Chat"),
            chunks[2]
        );
    }

    fn render_config_input(&self, f: &mut Frame, title: &str, _placeholder: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
            ].as_ref())
            .split(f.size());

        let display = if self.input.is_empty() {
            String::from("")
        } else {
            self.input.clone()
        };

        let input = Paragraph::new(display.as_str())
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(input, chunks[0]);

        let char_offset: u16 = self.input.chars().take(self.cursor_char_pos).map(display_width).sum::<usize>() as u16;
        let cursor_x = chunks[0].x + 1 + char_offset;
        let cursor_y = chunks[0].y + 1;
        f.set_cursor(cursor_x, cursor_y);

        f.render_widget(
            Paragraph::new("[ENTER] Save | [ESC] Cancel | Type to edit"),
            chunks[1]
        );
    }

    fn render_fetching_models(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
            ].as_ref())
            .split(f.size());

        let current_provider = self.config.get_provider();
        let message = if let Some(ref error) = self.model_fetch_error {
            format!("Error fetching models from {}: {}", current_provider, error)
        } else {
            format!("Fetching models from {}...", current_provider)
        };

        f.render_widget(
            Paragraph::new(message)
                .block(Block::default().borders(Borders::ALL).title("Fetch Models")),
            chunks[0]
        );

        f.render_widget(
            Paragraph::new("[ESC] Cancel"),
            chunks[1]
        );
    }

    fn render_select_model(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ].as_ref())
            .split(f.size());

        let current_provider = self.config.get_provider();
        f.render_widget(
            Paragraph::new(format!("Select model for {}", current_provider))
                .block(Block::default().borders(Borders::ALL).title("Select Model")),
            chunks[0]
        );

        let items: Vec<String> = self.fetched_models.clone();
        let list = Paragraph::new(Text::from(items.join("\n")))
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(list, chunks[1]);

        f.render_widget(
            Paragraph::new("[ENTER] Select | [ESC] Cancel"),
            chunks[2]
        );
    }

    fn handle_config_provider_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('1') => { self.config.set_provider("openai"); }
            KeyCode::Char('2') => { self.config.set_provider("anthropic"); }
            KeyCode::Char('3') => { self.config.set_provider("ollama"); }
            KeyCode::Char('4') => { self.config.set_provider("deepseek"); }
            KeyCode::Char('5') => { self.config.set_provider("minimax"); }
            KeyCode::Char('c') | KeyCode::Char('C') => { self.mode = AppMode::ConfigCustomName; }
            KeyCode::Char('f') | KeyCode::Char('F') => { self.fetch_models_from_provider(); }
            KeyCode::Esc => { self.mode = AppMode::Chat; }
            _ => {}
        }
        Ok(())
    }

    fn handle_config_input_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.insert_char_at_cursor(c);
            }
            KeyCode::Backspace => {
                self.remove_char_before_cursor();
            }
            KeyCode::Delete => {
                self.remove_char_at_cursor();
            }
            KeyCode::Left => {
                self.move_cursor_left();
            }
            KeyCode::Right => {
                self.move_cursor_right();
            }
            KeyCode::Home => {
                self.move_cursor_to_start();
            }
            KeyCode::End => {
                self.move_cursor_to_end();
            }
            KeyCode::Enter => {
                self.save_config_input();
                self.mode = AppMode::ConfigProvider;
            }
            KeyCode::Esc => {
                self.input.clear();
                self.cursor_char_pos = 0;
                self.mode = AppMode::ConfigProvider;
            }
            _ => {}
        }
        Ok(())
    }

    fn save_config_input(&mut self) {
        let provider = self.config.get_provider().to_string();
        match self.mode {
            AppMode::ConfigApiKey => {
                self.config.set_api_key(&provider, Some(self.input.clone()));
            }
            AppMode::ConfigModel => {
                self.config.set_model(&provider, self.input.clone());
            }
            AppMode::ConfigUrl => {
                self.config.set_base_url(&provider, Some(self.input.clone()));
            }
            AppMode::ConfigCustomName => {}
            AppMode::ConfigCustomUrl => {}
            _ => {}
        }
        self.input.clear();
        self.cursor_char_pos = 0;
    }

    fn fetch_models_from_provider(&mut self) {
        let provider_name = self.config.get_provider().to_string();
        let api_key_opt = self.config.get_api_key(&provider_name);
        let base_url_opt = self.config.get_base_url(&provider_name);
        let model_opt = Some(self.config.get_model(&provider_name));

        let (tx, rx) = mpsc::channel();
        self.model_fetch_receiver = Some(rx);
        self.mode = AppMode::ConfigFetchModels;

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = match provider_name.as_str() {
                "openai" => {
                    let p = OpenAIProvider::new(api_key_opt.unwrap_or_default(), base_url_opt);
                    p.list_models().await
                }
                "anthropic" => {
                    let p = AnthropicProvider::new(api_key_opt.unwrap_or_default(), base_url_opt);
                    p.list_models().await
                }
                "ollama" => {
                    let p = OllamaProvider::new(base_url_opt, model_opt);
                    p.list_models().await
                }
                "deepseek" => {
                    let p = DeepSeekProvider::new(api_key_opt.unwrap_or_default(), base_url_opt, model_opt);
                    p.list_models().await
                }
                "minimax" => {
                    let p = MiniMaxProvider::new(api_key_opt.unwrap_or_default(), base_url_opt, model_opt);
                    p.list_models().await
                }
                "custom" => {
                    let name = "custom".to_string();
                    let p = CustomProvider::new(name, api_key_opt.unwrap_or_default(), base_url_opt.unwrap_or_default(), model_opt.unwrap_or_default());
                    p.list_models().await
                }
                _ => Err(anyhow::anyhow!("Unknown provider")),
            };

            let _ = tx.send(result);
        });
    }

    fn handle_fetch_models_key(&mut self, key: event::KeyEvent) -> Result<()> {
        if let KeyCode::Esc = key.code {
            self.model_fetch_receiver = None;
            self.model_fetch_error = None;
            self.mode = AppMode::ConfigProvider;
        }
        Ok(())
    }

    fn handle_select_model_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.fetched_models.clear();
                self.mode = AppMode::ConfigProvider;
            }
            _ => {}
        }
        Ok(())
    }
}
