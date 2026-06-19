use crate::error::ApiError;
use crate::provider::Provider;
use crate::requests::headers::insert_header;
use crate::requests::headers::subagent_header;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::SessionSource;
use http::HeaderMap;
use serde_json::Value;
use serde_json::json;

/// Assembled request body plus headers for Chat Completions streaming calls.
pub struct ChatRequest {
    pub body: Value,
    pub headers: HeaderMap,
}

pub struct ChatRequestBuilder<'a> {
    model: &'a str,
    instructions: &'a str,
    input: &'a [ResponseItem],
    tools: &'a [Value],
    conversation_id: Option<String>,
    session_source: Option<SessionSource>,
}

impl<'a> ChatRequestBuilder<'a> {
    pub fn new(
        model: &'a str,
        instructions: &'a str,
        input: &'a [ResponseItem],
        tools: &'a [Value],
    ) -> Self {
        Self {
            model,
            instructions,
            input,
            tools,
            conversation_id: None,
            session_source: None,
        }
    }

    pub fn conversation_id(mut self, id: Option<String>) -> Self {
        self.conversation_id = id;
        self
    }

    pub fn session_source(mut self, source: Option<SessionSource>) -> Self {
        self.session_source = source;
        self
    }

    pub fn build(self, _provider: &Provider) -> Result<ChatRequest, ApiError> {
        let mut messages = Vec::new();

        // System prompt
        if !self.instructions.is_empty() {
            messages.push(json!({
                "role": "system",
                "content": self.instructions,
            }));
        }

        // User and assistant messages from input items
        for item in self.input {
            match item {
                ResponseItem::Message { role, content, .. } => {
                    let chat_role = match role.as_str() {
                        "user" => "user",
                        "assistant" => "assistant",
                        "system" => "system",
                        _ => "user",
                    };

                    let content_str = content
                        .iter()
                        .filter_map(|c| match c {
                            ContentItem::InputText { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    if !content_str.is_empty() {
                        messages.push(json!({
                            "role": chat_role,
                            "content": content_str,
                        }));
                    }
                }
                ResponseItem::FunctionCall { call_id, name, arguments, .. } => {
                    // DashScope requires arguments as a JSON object, not a string.
                    let args_value: Value = serde_json::from_str(arguments)
                        .unwrap_or_else(|_| json!({ "_raw": arguments }));
                    messages.push(json!({
                        "role": "assistant",
                        "tool_calls": [{
                            "id": call_id,
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": args_value,
                            }
                        }]
                    }));
                }
                ResponseItem::FunctionCallOutput { call_id, output, .. } => {
                    let output_str = match &output.body {
                        codex_protocol::models::FunctionCallOutputBody::Text(text) => {
                            text.clone()
                        }
                        codex_protocol::models::FunctionCallOutputBody::ContentItems(items) => items
                            .iter()
                            .filter_map(|c| match c {
                                FunctionCallOutputContentItem::InputText { text } => {
                                    Some(text.as_str())
                                }
                                FunctionCallOutputContentItem::InputImage { image_url, .. } => {
                                    Some(image_url.as_str())
                                }
                                FunctionCallOutputContentItem::EncryptedContent { .. } => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n"),
                    };

                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call_id,
                        "content": output_str,
                    }));
                }
                ResponseItem::Reasoning { content, .. } => {
                    if let Some(contents) = content {
                        for c in contents {
                            match c {
                                ReasoningItemContent::ReasoningText { text } => {
                                    messages.push(json!({
                                        "role": "system",
                                        "content": format!("Thinking: {}", text),
                                    }));
                                }
                                ReasoningItemContent::Text { text } => {
                                    messages.push(json!({
                                        "role": "system",
                                        "content": format!("Thinking: {}", text),
                                    }));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });

        // Add tools if present
        if !self.tools.is_empty() {
            let chat_tools = self
                .tools
                .iter()
                .map(|tool| {
                    if let Some(map) = tool.as_object() {
                        let _name = map
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        json!({
                            "type": "function",
                            "function": map,
                        })
                    } else {
                        json!({"type": "function", "function": {}})
                    }
                })
                .collect::<Vec<_>>();

            body["tools"] = serde_json::Value::Array(chat_tools);
        }

        // Build headers
        let mut headers = HeaderMap::new();

        if let Some(_id) = &self.conversation_id {
            // conversation headers
        }

        if let Some(_source) = &self.session_source {
            if let Some(header_value) = subagent_header(&self.session_source) {
                insert_header(&mut headers, "x-subagent", &header_value);
            }
        }

        Ok(ChatRequest { body, headers })
    }
}
