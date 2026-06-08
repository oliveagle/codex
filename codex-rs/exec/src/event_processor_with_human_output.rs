use std::io::IsTerminal;
use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;

use codex_app_server_protocol::CommandExecutionStatus;
use codex_app_server_protocol::McpToolCallStatus;
use codex_app_server_protocol::PatchApplyStatus;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadTokenUsage;
use codex_app_server_protocol::TurnStatus;
use codex_core::config::Config;
use codex_model_provider_info::WireApi;
use codex_protocol::num_format::format_with_separators;
use codex_protocol::protocol::SessionConfiguredEvent;
use codex_utils_sandbox_summary::summarize_permission_profile;
use owo_colors::OwoColorize;
use owo_colors::Style;

use crate::event_processor::CodexStatus;
use crate::event_processor::EventProcessor;
use crate::event_processor::handle_last_message;

/// Phase suffix for a message type (started vs completed)
enum MsgPhase {
    Req,  // "(req)" — tool call initiated
    Resp, // "(resp)" — tool call completed
}

impl MsgPhase {
    fn suffix(&self) -> &'static str {
        match self {
            MsgPhase::Req => "(req)",
            MsgPhase::Resp => "(resp)",
        }
    }
}

/// Message type for display formatting
enum MsgType {
    User,
    Assistant,
    Exec,
    Mcp,
    Collab,
    WebSearch,
    FileChange,
    Error,
    Stats,
    Info,
    Reasoning,
    Warning,
}

impl MsgType {
    fn base_label(&self) -> &'static str {
        match self {
            MsgType::User => "user",
            MsgType::Assistant => "assistant",
            MsgType::Exec => "exec",
            MsgType::Mcp => "mcp",
            MsgType::Collab => "collab",
            MsgType::WebSearch => "web_search",
            MsgType::FileChange => "file_change",
            MsgType::Error => "ERROR",
            MsgType::Stats => "STATS",
            MsgType::Info => "info",
            MsgType::Reasoning => "thinking",
            MsgType::Warning => "warning",
        }
    }

    fn label(&self, phase: Option<&MsgPhase>) -> String {
        match phase {
            Some(phase) => format!("{}{}", self.base_label(), phase.suffix()),
            None => self.base_label().to_string(),
        }
    }

    fn style(&self, proc: &EventProcessorWithHumanOutput) -> Style {
        match self {
            MsgType::User => proc.cyan,
            MsgType::Assistant => proc.green,
            MsgType::Exec => proc.yellow,
            MsgType::Mcp => proc.cyan,
            MsgType::Collab => proc.yellow,
            MsgType::WebSearch => proc.cyan,
            MsgType::FileChange => proc.dimmed,
            MsgType::Error => proc.red,
            MsgType::Stats => proc.magenta,
            MsgType::Info => proc.dimmed,
            MsgType::Reasoning => proc.dimmed,
            MsgType::Warning => proc.yellow,
        }
    }

    fn is_bold(&self, phase: Option<&MsgPhase>) -> bool {
        matches!(self, MsgType::Error | MsgType::Stats)
            || matches!(phase, Some(MsgPhase::Resp))
    }
}

pub(crate) struct EventProcessorWithHumanOutput {
    bold: Style,
    cyan: Style,
    dimmed: Style,
    green: Style,
    italic: Style,
    magenta: Style,
    red: Style,
    yellow: Style,
    show_agent_reasoning: bool,
    show_raw_agent_reasoning: bool,
    last_message_path: Option<PathBuf>,
    final_message: Option<String>,
    final_message_rendered: bool,
    emit_final_message_on_shutdown: bool,
    last_total_token_usage: Option<ThreadTokenUsage>,
    turn_start_time: Option<SystemTime>,
    /// Track the current reasoning item being streamed so that we know when to
    /// emit a new section header for subsequent deltas.
    current_reasoning_item_id: Option<String>,
    /// Counters for exec session stats
    command_exec_count: u64,
    mcp_tool_call_count: u64,
    collab_tool_call_count: u64,
    file_change_count: u64,
    web_search_count: u64,
}

/// Format current time as HH:MM:SS
fn format_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hh = (secs / 3600) % 24;
    let mm = (secs / 60) % 60;
    let ss = secs % 60;
    format!("{:02}:{:02}:{:02}", hh, mm, ss)
}

/// Format short hash from item id
fn short_hash(id: &str) -> String {
    if id.len() > 8 {
        id[..8].to_string()
    } else {
        id.to_string()
    }
}

impl EventProcessorWithHumanOutput {
    /// Format message prefix: TYPE (short_hash) [HH:MM:SS]
    fn format_msg_prefix(&self, msg_type: MsgType, phase: Option<MsgPhase>, id: Option<&str>) -> String {
        let style = msg_type.style(self);
        let label = msg_type.label(phase.as_ref());
        let hash = id.map_or(String::new(), |id| format!(" ({})", short_hash(id)));
        let timestamp = format_timestamp();
        let prefix = format!("{}{} [{}]", label, hash, timestamp);
        if msg_type.is_bold(phase.as_ref()) {
            prefix.style(style).bold().to_string()
        } else {
            prefix.style(style).to_string()
        }
    }

    pub(crate) fn create_with_ansi(
        with_ansi: bool,
        config: &Config,
        last_message_path: Option<PathBuf>,
    ) -> Self {
        let style = |styled: Style, plain: Style| if with_ansi { styled } else { plain };
        Self {
            bold: style(Style::new().bold(), Style::new()),
            cyan: style(Style::new().cyan(), Style::new()),
            dimmed: style(Style::new().dimmed(), Style::new()),
            green: style(Style::new().green(), Style::new()),
            italic: style(Style::new().italic(), Style::new()),
            magenta: style(Style::new().magenta(), Style::new()),
            red: style(Style::new().red(), Style::new()),
            yellow: style(Style::new().yellow(), Style::new()),
            show_agent_reasoning: !config.hide_agent_reasoning,
            show_raw_agent_reasoning: config.show_raw_agent_reasoning,
            last_message_path,
            final_message: None,
            final_message_rendered: false,
            emit_final_message_on_shutdown: false,
            last_total_token_usage: None,
            turn_start_time: None,
            current_reasoning_item_id: None,
            command_exec_count: 0,
            mcp_tool_call_count: 0,
            collab_tool_call_count: 0,
            file_change_count: 0,
            web_search_count: 0,
        }
    }

    fn render_item_started(&self, item: &ThreadItem) {
        match item {
            ThreadItem::CommandExecution { command, cwd, id, .. } => {
                eprintln!(
                    "{}\n{} in {cwd}",
                    "exec".style(self.italic).style(self.magenta),
                    command.style(self.bold),
                );
            }
            ThreadItem::McpToolCall { server, tool, id, .. } => {
                eprintln!(
                    "{} {} {}",
                    self.format_msg_prefix(MsgType::Mcp, Some(MsgPhase::Req), Some(id)),
                    format!("{server}/{tool}").style(self.cyan),
                    "started".style(self.dimmed)
                );
            }
            ThreadItem::WebSearch { query, id, .. } => {
                eprintln!(
                    "{} {}",
                    self.format_msg_prefix(MsgType::WebSearch, Some(MsgPhase::Req), Some(id)),
                    query
                );
            }
            ThreadItem::FileChange { id, .. } => {
                eprintln!("{}", self.format_msg_prefix(MsgType::FileChange, Some(MsgPhase::Req), Some(id)));
            }
            ThreadItem::CollabAgentToolCall { id, tool, .. } => {
                eprintln!(
                    "{} {:?}",
                    self.format_msg_prefix(MsgType::Collab, Some(MsgPhase::Req), Some(id)),
                    tool
                );
            }
            _ => {}
        }
    }

    fn render_item_completed(&mut self, item: ThreadItem) {
        match item {
            ThreadItem::AgentMessage { text, id, .. } => {
                eprintln!("{}", self.format_msg_prefix(MsgType::Assistant, None, Some(&id)));
                eprintln!("{}", text);
                self.final_message = Some(text);
                self.final_message_rendered = true;
            }
            ThreadItem::Reasoning {
                summary,
                content,
                id,
                ..
            } => {
                if self.show_agent_reasoning
                    && let Some(text) =
                        reasoning_text(&summary, &content, self.show_raw_agent_reasoning)
                    && !text.trim().is_empty()
                {
                    eprintln!("{}", self.format_msg_prefix(MsgType::Reasoning, None, Some(&id)));
                    eprintln!("{}", text);
                }
            }
            ThreadItem::CommandExecution {
                command: _,
                aggregated_output,
                exit_code,
                status,
                duration_ms,
                id,
                ..
            } => {
                self.command_exec_count += 1;
                let duration_suffix = duration_ms
                    .map(|duration_ms| format!(" in {duration_ms}ms"))
                    .unwrap_or_default();
                let status_label = match status {
                    CommandExecutionStatus::Completed => format!(" succeeded{}", duration_suffix),
                    CommandExecutionStatus::Failed => {
                        let exit_code = exit_code.unwrap_or(1);
                        format!(" exited {exit_code}{}", duration_suffix)
                    }
                    CommandExecutionStatus::Declined => {
                        format!(" declined{}", duration_suffix)
                    }
                    CommandExecutionStatus::InProgress => {
                        format!(" in progress{}", duration_suffix)
                    }
                };
                eprintln!(
                    "{}: {}",
                    self.format_msg_prefix(MsgType::Exec, Some(MsgPhase::Resp), Some(&id)),
                    status_label.style(self.green)
                );
                if let Some(output) = aggregated_output
                    && !output.trim().is_empty()
                {
                    eprintln!("{output}");
                }
            }
            ThreadItem::FileChange {
                changes, status, id, ..
            } => {
                self.file_change_count += 1;
                let status_text = match status {
                    PatchApplyStatus::Completed => "completed",
                    PatchApplyStatus::Failed => "failed",
                    PatchApplyStatus::Declined => "declined",
                    PatchApplyStatus::InProgress => "in_progress",
                };
                eprintln!(
                    "{} {}",
                    self.format_msg_prefix(MsgType::FileChange, Some(MsgPhase::Resp), Some(&id)),
                    status_text
                );
                for change in changes {
                    eprintln!("{}", change.path.style(self.dimmed));
                }
            }
            ThreadItem::McpToolCall {
                server,
                tool,
                status,
                error,
                id,
                ..
            } => {
                self.mcp_tool_call_count += 1;
                let status_text = match status {
                    McpToolCallStatus::Completed => "completed".style(self.green),
                    McpToolCallStatus::Failed => "failed".style(self.red),
                    McpToolCallStatus::InProgress => "in_progress".style(self.dimmed),
                };
                eprintln!(
                    "{} {} ({status_text})",
                    self.format_msg_prefix(MsgType::Mcp, Some(MsgPhase::Resp), Some(&id)),
                    format!("{server}/{tool}").style(self.cyan),
                );
                if let Some(error) = error {
                    eprintln!("{}", error.message.style(self.red));
                }
            }
            ThreadItem::WebSearch { query, id, .. } => {
                self.web_search_count += 1;
                eprintln!(
                    "{} {}",
                    self.format_msg_prefix(MsgType::WebSearch, Some(MsgPhase::Resp), Some(&id)),
                    query
                );
            }
            ThreadItem::ContextCompaction { id, .. } => {
                eprintln!("{}", self.format_msg_prefix(MsgType::Info, None, Some(&id)));
                eprintln!("{}", "context compacted".style(self.dimmed));
            }
            _ => {}
        }
    }
}

impl EventProcessor for EventProcessorWithHumanOutput {
    fn print_config_summary(
        &mut self,
        config: &Config,
        prompt: &str,
        session_configured_event: &SessionConfiguredEvent,
    ) {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        eprintln!("OpenAI Codex v{VERSION}\n--------");
        for (key, value) in config_summary_entries(config, session_configured_event) {
            eprintln!("{} {}", format!("{key}:").style(self.bold), value);
        }
        eprintln!("--------");
        eprintln!("{}", self.format_msg_prefix(MsgType::User, None, None));
        eprintln!("{}", prompt);
    }

    fn process_server_notification(&mut self, notification: ServerNotification) -> CodexStatus {
        match notification {
            ServerNotification::ConfigWarning(notification) => {
                let details = notification
                    .details
                    .map(|details| format!(" ({details})"))
                    .unwrap_or_default();
                eprintln!(
                    "{} {}{}",
                    self.format_msg_prefix(MsgType::Warning, None, None),
                    notification.summary,
                    details
                );
                CodexStatus::Running
            }
            ServerNotification::Warning(notification) => self.process_warning(notification.message),
            ServerNotification::Error(notification) => {
                eprintln!(
                    "{} {}",
                    self.format_msg_prefix(MsgType::Error, None, None),
                    notification.error
                );
                CodexStatus::Running
            }
            ServerNotification::DeprecationNotice(notification) => {
                eprintln!(
                    "{} {}",
                    self.format_msg_prefix(MsgType::Warning, None, None),
                    notification.summary
                );
                if let Some(details) = notification.details {
                    eprintln!("{}", details.style(self.dimmed));
                }
                CodexStatus::Running
            }
            ServerNotification::HookStarted(notification) => {
                eprintln!("{}", self.format_msg_prefix(MsgType::Info, None, None));
                eprintln!(
                    "{} {}",
                    "hook:".style(self.bold),
                    format!("{:?}", notification.run.event_name).style(self.dimmed)
                );
                CodexStatus::Running
            }
            ServerNotification::HookCompleted(notification) => {
                eprintln!("{}", self.format_msg_prefix(MsgType::Info, None, None));
                eprintln!(
                    "{} {} {:?}",
                    "hook:".style(self.bold),
                    format!("{:?}", notification.run.event_name).style(self.dimmed),
                    notification.run.status
                );
                CodexStatus::Running
            }
            ServerNotification::ItemStarted(notification) => {
                self.render_item_started(&notification.item);
                CodexStatus::Running
            }
            ServerNotification::ItemCompleted(notification) => {
                self.render_item_completed(notification.item);
                CodexStatus::Running
            }
            ServerNotification::ModelRerouted(notification) => {
                eprintln!(
                    "{} {} -> {}",
                    self.format_msg_prefix(MsgType::Warning, None, None),
                    notification.from_model,
                    notification.to_model
                );
                CodexStatus::Running
            }
            ServerNotification::ModelVerification(_) => CodexStatus::Running,
            ServerNotification::ThreadTokenUsageUpdated(notification) => {
                self.last_total_token_usage = Some(notification.token_usage);
                CodexStatus::Running
            }
            ServerNotification::TurnStarted(_) => {
                self.turn_start_time = Some(SystemTime::now());
                CodexStatus::Running
            }
            ServerNotification::TurnCompleted(notification) => match notification.turn.status {
                TurnStatus::Completed => {
                    let rendered_message = self
                        .final_message_rendered
                        .then(|| self.final_message.clone())
                        .flatten();
                    if let Some(final_message) =
                        final_message_from_turn_items(notification.turn.items.as_slice())
                    {
                        self.final_message_rendered =
                            rendered_message.as_deref() == Some(final_message.as_str());
                        self.final_message = Some(final_message);
                    }
                    self.emit_final_message_on_shutdown = true;
                    CodexStatus::InitiateShutdown
                }
                TurnStatus::Failed => {
                    self.final_message = None;
                    self.final_message_rendered = false;
                    self.emit_final_message_on_shutdown = false;
                    if let Some(error) = notification.turn.error {
                        eprintln!(
                            "{} {}",
                            self.format_msg_prefix(MsgType::Error, None, None),
                            error
                        );
                    }
                    CodexStatus::InitiateShutdown
                }
                TurnStatus::Interrupted => {
                    self.final_message = None;
                    self.final_message_rendered = false;
                    self.emit_final_message_on_shutdown = false;
                    eprintln!("{}", self.format_msg_prefix(MsgType::Info, None, None));
                    eprintln!("{}", "turn interrupted".style(self.dimmed));
                    CodexStatus::InitiateShutdown
                }
                TurnStatus::InProgress => CodexStatus::Running,
            },
            ServerNotification::TurnDiffUpdated(notification) => {
                if !notification.diff.trim().is_empty() {
                    eprintln!("{}", notification.diff);
                }
                CodexStatus::Running
            }
            ServerNotification::TurnPlanUpdated(notification) => {
                if let Some(explanation) = notification.explanation {
                    eprintln!("{}", explanation.style(self.italic));
                }
                for step in notification.plan {
                    match step.status {
                        codex_app_server_protocol::TurnPlanStepStatus::Completed => {
                            eprintln!("  {} {}", "✓".style(self.green), step.step);
                        }
                        codex_app_server_protocol::TurnPlanStepStatus::InProgress => {
                            eprintln!("  {} {}", "→".style(self.cyan), step.step);
                        }
                        codex_app_server_protocol::TurnPlanStepStatus::Pending => {
                            eprintln!(
                                "  {} {}",
                                "•".style(self.dimmed),
                                step.step.style(self.dimmed)
                            );
                        }
                    }
                }
                CodexStatus::Running
            }
            ServerNotification::ReasoningSummaryTextDelta(notification) => {
                if self.show_agent_reasoning && !notification.delta.is_empty() {
                    if self.current_reasoning_item_id.as_deref()
                        != Some(notification.item_id.as_str())
                    {
                        self.current_reasoning_item_id = Some(notification.item_id.clone());
                        eprintln!(
                            "{}",
                            self.format_msg_prefix(MsgType::Reasoning, None, Some(&notification.item_id))
                        );
                    }
                    eprint!("{}", notification.delta);
                    let _ = std::io::stderr().flush();
                }
                CodexStatus::Running
            }
            ServerNotification::ReasoningTextDelta(notification) => {
                if self.show_agent_reasoning
                    && self.show_raw_agent_reasoning
                    && !notification.delta.is_empty()
                {
                    if self.current_reasoning_item_id.as_deref()
                        != Some(notification.item_id.as_str())
                    {
                        self.current_reasoning_item_id = Some(notification.item_id.clone());
                        eprintln!(
                            "{}",
                            self.format_msg_prefix(MsgType::Reasoning, None, Some(&notification.item_id))
                        );
                    }
                    eprint!("{}", notification.delta);
                    let _ = std::io::stderr().flush();
                }
                CodexStatus::Running
            }
            ServerNotification::ReasoningSummaryPartAdded(_) => {
                if self.show_agent_reasoning {
                    // End the current streaming section so subsequent deltas
                    // start a fresh block with a new prefix header.
                    eprintln!();
                    self.current_reasoning_item_id = None;
                }
                CodexStatus::Running
            }
            _ => CodexStatus::Running,
        }
    }

    fn process_warning(&mut self, message: String) -> CodexStatus {
        eprintln!(
            "{} {message}",
            self.format_msg_prefix(MsgType::Warning, None, None),
        );
        CodexStatus::Running
    }

    fn print_final_output(&mut self) {
        if self.emit_final_message_on_shutdown
            && let Some(path) = self.last_message_path.as_deref()
        {
            handle_last_message(self.final_message.as_deref(), path);
        }

        // Compute wall-clock duration for this turn (fallback to 0 when no turn
        // start was recorded, which can happen for empty sessions).
        let elapsed_secs = self
            .turn_start_time
            .and_then(|t| t.elapsed().ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        // Aggregate tool-call counts so the user can see how busy the turn was
        // at a glance, broken down by category (shell vs MCP vs web search).
        let total_tool_calls = self.command_exec_count
            + self.mcp_tool_call_count
            + self.collab_tool_call_count
            + self.web_search_count;

        eprintln!("{}", self.format_msg_prefix(MsgType::Stats, None, None));
        eprintln!("  duration:      {}", format_duration(elapsed_secs));
        eprintln!("  tool calls:    {} (shell: {}, mcp: {}, collab: {}, web_search: {})",
            total_tool_calls,
            self.command_exec_count,
            self.mcp_tool_call_count,
            self.collab_tool_call_count,
            self.web_search_count,
        );
        eprintln!("  file changes:  {}", self.file_change_count);

        if let Some(usage) = &self.last_total_token_usage {
            let total = usage.total.total_tokens;
            let input = usage.total.input_tokens;
            let output = usage.total.output_tokens;
            let cached = usage.total.cached_input_tokens;
            let reasoning = usage.total.reasoning_output_tokens;

            eprintln!("  tokens used:   {} (input: {}, cached: {}, output: {}, reasoning: {})",
                format_with_separators(total as i64),
                format_with_separators(input as i64),
                format_with_separators(cached as i64),
                format_with_separators(output as i64),
                format_with_separators(reasoning as i64)
            );

            // Calculate throughput if we have timing data
            if elapsed_secs > 0.0 {
                let prefill_rate = (input - cached) as f64 / elapsed_secs;
                let decode_rate = (output + reasoning) as f64 / elapsed_secs;
                eprintln!("  throughput:    prefill {:.0} tok/s, decode {:.0} tok/s",
                    prefill_rate, decode_rate
                );
            }
        }

        #[allow(clippy::print_stdout)]
        if should_print_final_message_to_stdout(
            self.emit_final_message_on_shutdown
                .then_some(self.final_message.as_deref())
                .flatten(),
            std::io::stdout().is_terminal(),
            std::io::stderr().is_terminal(),
        ) && let Some(message) = self.final_message.as_deref()
        {
            println!("{message}");
        } else if should_print_final_message_to_tty(
            self.emit_final_message_on_shutdown
                .then_some(self.final_message.as_deref())
                .flatten(),
            self.final_message_rendered,
            std::io::stdout().is_terminal(),
            std::io::stderr().is_terminal(),
        ) && let Some(message) = self.final_message.as_deref()
        {
            eprintln!("{}", message);
        }
    }
}

/// Render a wall-clock duration as a human-readable string, picking the
/// largest unit that keeps the leading value non-zero.
fn format_duration(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "unknown".to_string();
    }
    let total_ms = (secs * 1000.0).round() as u64;
    if total_ms < 1000 {
        return format!("{total_ms}ms");
    }
    let total_secs = total_ms / 1000;
    if total_secs < 60 {
        let ms = total_ms % 1000;
        return format!("{}.{:03}s", total_secs, ms);
    }
    let minutes = total_secs / 60;
    let rem_secs = total_secs % 60;
    if minutes < 60 {
        return format!("{minutes}m {rem_secs}s");
    }
    let hours = minutes / 60;
    let rem_minutes = minutes % 60;
    format!("{hours}h {rem_minutes}m {rem_secs}s")
}

fn config_summary_entries(
    config: &Config,
    session_configured_event: &SessionConfiguredEvent,
) -> Vec<(&'static str, String)> {
    let permission_profile = config.permissions.effective_permission_profile();
    let mut entries = vec![
        ("workdir", config.cwd.display().to_string()),
        ("model", session_configured_event.model.clone()),
        (
            "provider",
            session_configured_event.model_provider_id.clone(),
        ),
        (
            "approval",
            config.permissions.approval_policy.value().to_string(),
        ),
        (
            "sandbox",
            summarize_permission_profile(
                &permission_profile,
                &config.cwd,
                config.effective_workspace_roots().as_slice(),
            ),
        ),
    ];
    if config.model_provider.wire_api == WireApi::Responses {
        entries.push((
            "reasoning effort",
            config
                .model_reasoning_effort
                .as_ref()
                .map(std::string::ToString::to_string)
                .unwrap_or_else(|| "none".to_string()),
        ));
        entries.push((
            "reasoning summaries",
            config
                .model_reasoning_summary
                .map(|summary| summary.to_string())
                .unwrap_or_else(|| "none".to_string()),
        ));
    }
    entries.push((
        "session id",
        session_configured_event.session_id.to_string(),
    ));
    entries
}

fn reasoning_text(
    summary: &[String],
    content: &[String],
    show_raw_agent_reasoning: bool,
) -> Option<String> {
    let entries = if show_raw_agent_reasoning && !content.is_empty() {
        content
    } else {
        summary
    };
    if entries.is_empty() {
        None
    } else {
        Some(entries.join("\n"))
    }
}

fn final_message_from_turn_items(items: &[ThreadItem]) -> Option<String> {
    items
        .iter()
        .rev()
        .find_map(|item| match item {
            ThreadItem::AgentMessage { text, .. } => Some(text.clone()),
            _ => None,
        })
        .or_else(|| {
            items.iter().rev().find_map(|item| match item {
                ThreadItem::Plan { text, .. } => Some(text.clone()),
                _ => None,
            })
        })
}

fn should_print_final_message_to_stdout(
    final_message: Option<&str>,
    stdout_is_terminal: bool,
    stderr_is_terminal: bool,
) -> bool {
    final_message.is_some() && !(stdout_is_terminal && stderr_is_terminal)
}

fn should_print_final_message_to_tty(
    final_message: Option<&str>,
    final_message_rendered: bool,
    stdout_is_terminal: bool,
    stderr_is_terminal: bool,
) -> bool {
    final_message.is_some() && !final_message_rendered && stdout_is_terminal && stderr_is_terminal
}

#[cfg(test)]
#[path = "event_processor_with_human_output_tests.rs"]
mod tests;
