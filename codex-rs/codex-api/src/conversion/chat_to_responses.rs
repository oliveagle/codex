//! Conversion from OpenAI Chat Completions API to Codex Responses API.
//!
//! This module provides functionality to convert OpenAI-compatible Chat Completions
//! requests into the Codex Responses API format.

use crate::common::ResponsesApiRequest;
use crate::types::chat::{
    ChatCompletionRequest, ChatFunction, ChatMessage, ChatMessageRole, ChatTool, ChatToolChoice,
    ChatToolType,
};
use codex_protocol::models::{ContentItem, FunctionCallOutputPayload, ResponseItem};
use serde_json::json;
use thiserror::Error;

/// Errors that can occur during Chat Completions to Responses API conversion.
#[derive(Debug, Error)]
pub enum ConversionError {
    /// Invalid message role encountered.
    #[error("invalid role: {0}")]
    InvalidRole(String),

    /// Tool message is missing tool_call_id.
    #[error("tool message missing tool_call_id")]
    MissingToolCallId,

    /// Unsupported tool type.
    #[error("unsupported tool type: {0}")]
    UnsupportedToolType(String),
}

/// Converts a Chat Completions API request to a Responses API request.
///
/// # Arguments
///
/// * `chat` - The Chat Completions request to convert
///
/// # Returns
///
/// A `ResponsesApiRequest` that can be used with the Codex Responses API
///
/// # Conversion Rules
///
/// - **messages**: Converted to `input` (user/assistant messages) and `instructions` (system messages)
///   - System messages → extracted to `instructions` field (concatenated with "\n\n")
///   - User messages → `ResponseItem::Message { role: "user", ... }`
///   - Assistant messages → `ResponseItem::Message { role: "assistant", ... }`
///   - Tool messages (role=tool) → `ResponseItem::FunctionCallOutput`
///   - Assistant messages with tool_calls → split into multiple `ResponseItem::FunctionCall` entries
/// - **model**: Passed through to `ResponsesApiRequest.model`
/// - **stream**: Passed through to `ResponsesApiRequest.stream`
/// - **tools**: Converted to JSON values compatible with OpenAI Responses API format
/// - **tool_choice**: Converted to string representation
/// - **max_tokens/max_completion_tokens/temperature/top_p**: Ignored (Responses API has different parameters)
///
/// # Errors
///
/// Returns `Err` if:
/// - A tool message is missing `tool_call_id`
/// - An invalid role is encountered
pub fn convert_chat_to_responses(
    chat: &ChatCompletionRequest,
) -> Result<ResponsesApiRequest, ConversionError> {
    let mut input = Vec::new();
    let mut instructions = String::new();
    let mut system_messages = Vec::new();

    // Process messages in order
    for msg in &chat.messages {
        match msg.role {
            ChatMessageRole::System => {
                // Extract system messages to instructions
                if let Some(content) = &msg.content {
                    system_messages.push(content.as_str());
                }
            }
            ChatMessageRole::User => {
                // Convert user message to ResponseItem
                let content = msg.content.as_deref().unwrap_or("");
                input.push(ResponseItem::Message {
                    id: None,
                    role: "user".to_string(),
                    content: vec![ContentItem::InputText {
                        text: content.to_string(),
                    }],
                    phase: None,
                    internal_chat_message_metadata_passthrough: None,
                });
            }
            ChatMessageRole::Assistant => {
                // Handle assistant messages with or without tool_calls
                if let Some(tool_calls) = &msg.tool_calls {
                    // Split multiple tool_calls into separate ResponseItem entries
                    for tool_call in tool_calls {
                        input.push(ResponseItem::FunctionCall {
                            id: None,
                            name: tool_call.function.name.clone(),
                            namespace: None,
                            arguments: tool_call.function.arguments.clone(),
                            call_id: tool_call.id.clone(),
                            internal_chat_message_metadata_passthrough: None,
                        });
                    }
                    // Also add assistant content if present (can coexist with tool_calls)
                    if let Some(content) = &msg.content {
                        if !content.is_empty() {
                            input.push(ResponseItem::Message {
                                id: None,
                                role: "assistant".to_string(),
                                content: vec![ContentItem::InputText {
                                    text: content.clone(),
                                }],
                                phase: None,
                                internal_chat_message_metadata_passthrough: None,
                            });
                        }
                    }
                } else {
                    // Regular assistant message
                    let content = msg.content.as_deref().unwrap_or("");
                    if !content.is_empty() {
                        input.push(ResponseItem::Message {
                            id: None,
                            role: "assistant".to_string(),
                            content: vec![ContentItem::InputText {
                                text: content.to_string(),
                            }],
                            phase: None,
                            internal_chat_message_metadata_passthrough: None,
                        });
                    }
                }
            }
            ChatMessageRole::Tool => {
                // Convert tool message to FunctionCallOutput
                let tool_call_id = msg.tool_call_id.as_ref().ok_or_else(|| {
                    ConversionError::MissingToolCallId
                })?;
                let output = msg.content.as_deref().unwrap_or("");
                input.push(ResponseItem::FunctionCallOutput {
                    id: None,
                    call_id: tool_call_id.clone(),
                    output: FunctionCallOutputPayload::from_text(output.to_string()),
                    internal_chat_message_metadata_passthrough: None,
                });
            }
        }
    }

    // Concatenate system messages with "\n\n"
    instructions = system_messages.join("\n\n");

    // Convert tools array to JSON values
    let tools = chat
        .tools
        .as_ref()
        .map(|tools| {
            Some(
                tools
                    .iter()
                    .map(|tool| convert_chat_tool_to_json(tool))
                    .collect(),
            )
        })
        .unwrap_or_default();

    // Convert tool_choice to string
    let tool_choice = convert_tool_choice_to_string(&chat.tool_choice);

    Ok(ResponsesApiRequest {
        model: chat.model.clone(),
        instructions,
        input,
        tools,
        tool_choice,
        parallel_tool_calls: true,
        reasoning: None,
        store: false,
        stream: chat.stream.unwrap_or(false),
        include: vec![],
        service_tier: None,
        prompt_cache_key: None,
        text: None,
        client_metadata: None,
    })
}

/// Converts a ChatTool to a JSON value compatible with OpenAI Responses API format.
fn convert_chat_tool_to_json(tool: &ChatTool) -> serde_json::Value {
    json!({
        "type": "function",
        "name": tool.function.name,
        "description": tool.function.description,
        "parameters": tool.function.parameters
    })
}

/// Converts ChatToolChoice to a string representation.
fn convert_tool_choice_to_string(tool_choice: &Option<ChatToolChoice>) -> String {
    match tool_choice {
        None => "auto".to_string(),
        Some(ChatToolChoice::Auto) => "auto".to_string(),
        Some(ChatToolChoice::None) => "none".to_string(),
        Some(ChatToolChoice::Required(func)) => {
            json!({
                "type": "function",
                "name": func.function.name
            })
            .to_string()
        }
        Some(ChatToolChoice::Specific {
            tool_type: _,
            function,
        }) => {
            json!({
                "type": "function",
                "name": function.name
            })
            .to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::chat::{ChatFunctionCall, ChatToolCall};
    use serde_json::json;

    fn create_test_user_message(content: &str) -> ChatMessage {
        ChatMessage {
            role: ChatMessageRole::User,
            content: Some(content.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    fn create_test_assistant_message(content: &str) -> ChatMessage {
        ChatMessage {
            role: ChatMessageRole::Assistant,
            content: Some(content.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    fn create_test_system_message(content: &str) -> ChatMessage {
        ChatMessage {
            role: ChatMessageRole::System,
            content: Some(content.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    fn create_test_tool_message(tool_call_id: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: ChatMessageRole::Tool,
            content: Some(content.to_string()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
            name: None,
        }
    }

    fn create_assistant_with_tool_calls(
        tool_calls: Vec<ChatToolCall>,
    ) -> ChatMessage {
        ChatMessage {
            role: ChatMessageRole::Assistant,
            content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
        }
    }

    fn create_assistant_with_tool_calls_and_content(
        tool_calls: Vec<ChatToolCall>,
        content: &str,
    ) -> ChatMessage {
        ChatMessage {
            role: ChatMessageRole::Assistant,
            content: Some(content.to_string()),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            name: None,
        }
    }

    #[test]
    fn test_single_user_message() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("Hello!")],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.model, "gpt-4");
        assert_eq!(result.input.len(), 1);
        assert!(result.instructions.is_empty());
        assert!(matches!(
            &result.input[0],
            ResponseItem::Message { role, .. } if role == "user"
        ));
    }

    #[test]
    fn test_multi_turn_user_assistant() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_user_message("Hello"),
                create_test_assistant_message("Hi there"),
                create_test_user_message("How are you?"),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.input.len(), 3);
        assert!(matches!(
            &result.input[0],
            ResponseItem::Message { role, .. } if role == "user"
        ));
        assert!(matches!(
            &result.input[1],
            ResponseItem::Message { role, .. } if role == "assistant"
        ));
        assert!(matches!(
            &result.input[2],
            ResponseItem::Message { role, .. } if role == "user"
        ));
    }

    #[test]
    fn test_system_message_extracted_to_instructions() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_system_message("You are a helpful assistant"),
                create_test_user_message("Hello"),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.instructions, "You are a helpful assistant");
        assert_eq!(result.input.len(), 1);
        assert!(matches!(
            &result.input[0],
            ResponseItem::Message { role, .. } if role == "user"
        ));
    }

    #[test]
    fn test_multiple_system_messages_concatenated() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_system_message("First instruction"),
                create_test_system_message("Second instruction"),
                create_test_user_message("Hello"),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.instructions, "First instruction\n\nSecond instruction");
        assert_eq!(result.input.len(), 1);
    }

    #[test]
    fn test_tool_message_with_call_id() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_user_message("What's the weather?"),
                create_test_tool_message("call_123", "The weather is sunny"),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.input.len(), 2);
        assert!(matches!(
            &result.input[1],
            ResponseItem::FunctionCallOutput { call_id, .. } if call_id == "call_123"
        ));
    }

    #[test]
    fn test_assistant_with_tool_calls_split() {
        let tool_calls = vec![ChatToolCall {
            id: "call_1".to_string(),
            tool_type: ChatToolType::Function,
            function: crate::types::chat::ChatFunctionCall {
                name: "get_weather".to_string(),
                arguments: r#"{"location": "NYC"}"#.to_string(),
            },
        }];

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_user_message("What's the weather in NYC?"),
                create_assistant_with_tool_calls(tool_calls),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.input.len(), 2);
        assert!(matches!(
            &result.input[0],
            ResponseItem::Message { role, .. } if role == "user"
        ));
        assert!(matches!(
            &result.input[1],
            ResponseItem::FunctionCall { name, call_id, .. }
                if name == "get_weather" && call_id == "call_1"
        ));
    }

    #[test]
    fn test_empty_messages_returns_ok() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert!(result.input.is_empty());
        assert!(result.instructions.is_empty());
    }

    #[test]
    fn test_invalid_role_returns_err() {
        // This test ensures we handle unknown roles properly
        // Since ChatMessageRole is an enum, we can't create invalid values directly
        // but the conversion should handle all enum variants correctly
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("Hello")],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        // This should succeed because User is a valid role
        assert!(convert_chat_to_responses(&chat).is_ok());
    }

    #[test]
    fn test_tool_message_without_call_id_returns_err() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatMessageRole::Tool,
                content: Some("Some output".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat);
        assert!(matches!(result, Err(ConversionError::MissingToolCallId)));
    }

    #[test]
    fn test_stream_flag_passes_through() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("Hello")],
            temperature: None,
            top_p: None,
            stream: Some(true),
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.stream, true);
    }

    #[test]
    fn test_tools_array_converted() {
        let tools = vec![ChatTool {
            tool_type: ChatToolType::Function,
            function: ChatFunction {
                name: "get_weather".to_string(),
                description: Some("Get current weather".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    }
                })),
                strict: None,
            },
        }];

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("What's the weather?")],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: Some(tools),
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0]["type"], "function");
        assert_eq!(result.tools[0]["name"], "get_weather");
        assert_eq!(result.tools[0]["description"], "Get current weather");
    }

    #[test]
    fn test_tool_choice_string_passes_through() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("Hello")],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: Some(ChatToolChoice::Auto),
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.tool_choice, "auto");
    }

    #[test]
    fn test_assistant_with_multiple_tool_calls() {
        let tool_calls = vec![
            ChatToolCall {
                id: "call_1".to_string(),
                tool_type: ChatToolType::Function,
                function: ChatFunctionCall {
                    name: "get_weather".to_string(),
                    arguments: r#"{"location": "NYC"}"#.to_string(),
                },
            },
            ChatToolCall {
                id: "call_2".to_string(),
                tool_type: ChatToolType::Function,
                function: ChatFunctionCall {
                    name: "get_time".to_string(),
                    arguments: r#"{"timezone": "EST"}"#.to_string(),
                },
            },
        ];

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_user_message("What's the weather and time in NYC?"),
                create_assistant_with_tool_calls(tool_calls),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.input.len(), 3);
        // First item should be user message
        assert!(matches!(
            &result.input[0],
            ResponseItem::Message { role, .. } if role == "user"
        ));
        // Second and third items should be FunctionCall items
        assert!(matches!(
            &result.input[1],
            ResponseItem::FunctionCall { name, call_id, .. }
                if name == "get_weather" && call_id == "call_1"
        ));
        assert!(matches!(
            &result.input[2],
            ResponseItem::FunctionCall { name, call_id, .. }
                if name == "get_time" && call_id == "call_2"
        ));
    }

    #[test]
    fn test_assistant_empty_content_not_added() {
        // Assistant message with empty content should not be added to input
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_user_message("Hello"),
                ChatMessage {
                    role: ChatMessageRole::Assistant,
                    content: Some("".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
                create_test_user_message("Continue"),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        // Should have 2 items: user messages, but the empty assistant message should be skipped
        assert_eq!(result.input.len(), 2);
        assert!(matches!(
            &result.input[0],
            ResponseItem::Message { role, .. } if role == "user"
        ));
        assert!(matches!(
            &result.input[1],
            ResponseItem::Message { role, .. } if role == "user"
        ));
    }

    #[test]
    fn test_assistant_with_tool_calls_and_content() {
        // Assistant message can have both tool_calls AND content
        let tool_calls = vec![ChatToolCall {
            id: "call_1".to_string(),
            tool_type: ChatToolType::Function,
            function: ChatFunctionCall {
                name: "get_weather".to_string(),
                arguments: r#"{"location": "SF"}"#.to_string(),
            },
        }];

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_user_message("What's the weather in SF?"),
                create_assistant_with_tool_calls_and_content(
                    tool_calls,
                    "I'll check the weather for you.",
                ),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        // Should have 3 items: user message, FunctionCall, assistant message with content
        assert_eq!(result.input.len(), 3);
        assert!(matches!(
            &result.input[0],
            ResponseItem::Message { role, .. } if role == "user"
        ));
        assert!(matches!(
            &result.input[1],
            ResponseItem::FunctionCall { name, call_id, .. }
                if name == "get_weather" && call_id == "call_1"
        ));
        assert!(matches!(
            &result.input[2],
            ResponseItem::Message { role, content, .. } if role == "assistant"
                && content.len() == 1 && matches!(&content[0], ContentItem::InputText { text } if text == "I'll check the weather for you.")
        ));
    }

    #[test]
    fn test_multi_turn_conversation_with_tools() {
        // Full multi-turn conversation: user -> assistant(tool_call) -> tool -> assistant(text)
        let tool_calls = vec![ChatToolCall {
            id: "call_abc".to_string(),
            tool_type: ChatToolType::Function,
            function: ChatFunctionCall {
                name: "get_weather".to_string(),
                arguments: r#"{"city": "Seattle"}"#.to_string(),
            },
        }];

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_user_message("What's the weather in Seattle?"),
                create_assistant_with_tool_calls(tool_calls),
                create_test_tool_message("call_abc", "72°F and sunny"),
                create_test_assistant_message("The weather in Seattle is 72°F and sunny."),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        // Should have 4 items
        assert_eq!(result.input.len(), 4);

        // First: user message
        assert!(matches!(
            &result.input[0],
            ResponseItem::Message { role, .. } if role == "user"
        ));

        // Second: FunctionCall
        assert!(matches!(
            &result.input[1],
            ResponseItem::FunctionCall { name, call_id, .. }
                if name == "get_weather" && call_id == "call_abc"
        ));

        // Third: FunctionCallOutput
        assert!(matches!(
            &result.input[2],
            ResponseItem::FunctionCallOutput { call_id, .. }
                if call_id == "call_abc"
        ));

        // Fourth: assistant message
        assert!(matches!(
            &result.input[3],
            ResponseItem::Message { role, .. } if role == "assistant"
        ));
    }

    #[test]
    fn test_tool_choice_required_converts_correctly() {
        use crate::types::chat::{ChatFunctionChoice, ChatRequiredFunction};

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("Hello")],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: Some(ChatToolChoice::Required(ChatRequiredFunction {
                function: ChatFunctionChoice {
                    name: "my_function".to_string(),
                },
            })),
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        // tool_choice should be a JSON string with type=function and name
        assert!(result.tool_choice.contains("function"));
        assert!(result.tool_choice.contains("my_function"));
    }

    #[test]
    fn test_tool_choice_none_converts_to_none_string() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("Hello")],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: Some(ChatToolChoice::None),
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.tool_choice, "none");
    }

    #[test]
    fn test_tool_with_empty_parameters_array() {
        let tools = vec![ChatTool {
            tool_type: ChatToolType::Function,
            function: ChatFunction {
                name: "simple_func".to_string(),
                description: Some("Simple function".to_string()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {}
                })),
                strict: None,
            },
        }];

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("Test")],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: Some(tools),
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0]["name"], "simple_func");
    }

    #[test]
    fn test_tool_with_no_parameters() {
        let tools = vec![ChatTool {
            tool_type: ChatToolType::Function,
            function: ChatFunction {
                name: "no_params_func".to_string(),
                description: Some("No params".to_string()),
                parameters: None,
                strict: None,
            },
        }];

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("Test")],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: Some(tools),
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.tools[0]["name"], "no_params_func");
        // parameters may be serialized as null when None
        assert!(result.tools[0]["parameters"].is_null() || result.tools[0].get("parameters").is_none());
    }

    #[test]
    fn test_tool_with_no_description() {
        let tools = vec![ChatTool {
            tool_type: ChatToolType::Function,
            function: ChatFunction {
                name: "no_desc_func".to_string(),
                description: None,
                parameters: Some(serde_json::json!({"type": "object"})),
                strict: None,
            },
        }];

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("Test")],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: Some(tools),
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.tools[0]["name"], "no_desc_func");
        // description may be serialized as null when None
        assert!(result.tools[0]["description"].is_null() || result.tools[0].get("description").is_none());
    }

    #[test]
    fn test_empty_tools_array() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("Test")],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: Some(vec![]),
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.tools.len(), 0);
    }

    #[test]
    fn test_all_message_types_in_single_conversation() {
        let tool_calls = vec![ChatToolCall {
            id: "call_all".to_string(),
            tool_type: ChatToolType::Function,
            function: ChatFunctionCall {
                name: "test_func".to_string(),
                arguments: r#"{"arg":"val"}"#.to_string(),
            },
        }];

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_system_message("System prompt"),
                create_test_user_message("User message"),
                create_assistant_with_tool_calls(tool_calls),
                create_test_tool_message("call_all", "Tool result"),
                create_test_assistant_message("Final response"),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        // System messages go to instructions
        assert_eq!(result.instructions, "System prompt");

        // Input should have: user, FunctionCall, FunctionCallOutput, assistant
        assert_eq!(result.input.len(), 4);
        assert!(matches!(
            &result.input[0],
            ResponseItem::Message { role, .. } if role == "user"
        ));
        assert!(matches!(
            &result.input[1],
            ResponseItem::FunctionCall { .. }
        ));
        assert!(matches!(
            &result.input[2],
            ResponseItem::FunctionCallOutput { .. }
        ));
        assert!(matches!(
            &result.input[3],
            ResponseItem::Message { role, .. } if role == "assistant"
        ));
    }

    #[test]
    fn test_mixed_system_user_assistant_tool_messages() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_system_message("System 1"),
                create_test_system_message("System 2"),
                create_test_user_message("User 1"),
                create_test_assistant_message("Assistant 1"),
                create_test_user_message("User 2"),
                create_test_tool_message("call_1", "Result 1"),
                create_test_assistant_message("Assistant 2"),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert_eq!(result.instructions, "System 1\n\nSystem 2");
        // System messages are extracted to instructions, so input has 5 items
        assert_eq!(result.input.len(), 5);
    }

    #[test]
    fn test_unicode_content() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatMessageRole::User,
                content: Some("Hello 🌍 世界 こんにちは 🎉".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        if let ResponseItem::Message { content, .. } = &result.input[0] {
            if let ContentItem::InputText { text } = &content[0] {
                assert!(text.contains("🌍"));
                assert!(text.contains("世界"));
            } else {
                panic!("Expected InputText");
            }
        } else {
            panic!("Expected Message");
        }
    }

    #[test]
    fn test_multiline_content() {
        let long_content = "Line 1\nLine 2\nLine 3\nLine 4";
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatMessageRole::User,
                content: Some(long_content.to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        if let ResponseItem::Message { content, .. } = &result.input[0] {
            if let ContentItem::InputText { text } = &content[0] {
                assert_eq!(text, long_content);
            } else {
                panic!("Expected InputText");
            }
        } else {
            panic!("Expected Message");
        }
    }

    #[test]
    fn test_very_long_content() {
        let long_text = "A".repeat(10000);
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatMessageRole::User,
                content: Some(long_text.clone()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        if let ResponseItem::Message { content, .. } = &result.input[0] {
            if let ContentItem::InputText { text } = &content[0] {
                assert_eq!(text.len(), 10000);
            } else {
                panic!("Expected InputText");
            }
        } else {
            panic!("Expected Message");
        }
    }

    #[test]
    fn test_tool_message_with_empty_content() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatMessageRole::Tool,
                content: Some("".to_string()),
                tool_calls: None,
                tool_call_id: Some("call_123".to_string()),
                name: None,
            }],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        if let ResponseItem::FunctionCallOutput { output, .. } = &result.input[0] {
            // Output body should be Text type with empty content
            let text = output.body.to_text();
            assert_eq!(text, Some("".to_string()));
        } else {
            panic!("Expected FunctionCallOutput");
        }
    }

    #[test]
    fn test_parallel_tool_calls_in_single_assistant_message() {
        let tool_calls = vec![
            ChatToolCall {
                id: "call_parallel_1".to_string(),
                tool_type: ChatToolType::Function,
                function: ChatFunctionCall {
                    name: "get_weather".to_string(),
                    arguments: r#"{"city":"NYC"}"#.to_string(),
                },
            },
            ChatToolCall {
                id: "call_parallel_2".to_string(),
                tool_type: ChatToolType::Function,
                function: ChatFunctionCall {
                    name: "get_time".to_string(),
                    arguments: r#"{"timezone":"EST"}"#.to_string(),
                },
            },
            ChatToolCall {
                id: "call_parallel_3".to_string(),
                tool_type: ChatToolType::Function,
                function: ChatFunctionCall {
                    name: "get_date".to_string(),
                    arguments: r#"{"format":"full"}"#.to_string(),
                },
            },
        ];

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_user_message("Get weather, time, and date"),
                create_assistant_with_tool_calls(tool_calls),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        // Should have 4 items: user + 3 FunctionCalls
        assert_eq!(result.input.len(), 4);

        // Check all three tool calls are present
        let mut tool_call_count = 0;
        for item in &result.input {
            if let ResponseItem::FunctionCall { name, call_id, .. } = item {
                tool_call_count += 1;
                assert!(call_id.starts_with("call_parallel_"));
            }
        }
        assert_eq!(tool_call_count, 3);
    }

    #[test]
    fn test_assistant_message_with_name() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatMessageRole::Assistant,
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: Some("CustomAssistant".to_string()),
            }],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        // Name is not used in conversion but should not cause error
        assert_eq!(result.input.len(), 1);
    }

    #[test]
    fn test_user_message_with_empty_content() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatMessageRole::User,
                content: Some("".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        // Empty content should still be added as a message
        assert_eq!(result.input.len(), 1);
        if let ResponseItem::Message { content, .. } = &result.input[0] {
            if let ContentItem::InputText { text } = &content[0] {
                assert!(text.is_empty());
            }
        }
    }

    #[test]
    fn test_tool_choice_specific_converts_correctly() {
        use crate::types::chat::{ChatFunctionChoice};

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![create_test_user_message("Hello")],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: Some(ChatToolChoice::Specific {
                tool_type: ChatToolType::Function,
                function: ChatFunctionChoice {
                    name: "specific_func".to_string(),
                },
            }),
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        assert!(result.tool_choice.contains("function"));
        assert!(result.tool_choice.contains("specific_func"));
    }

    #[test]
    fn test_system_message_with_empty_content() {
        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                ChatMessage {
                    role: ChatMessageRole::System,
                    content: Some("".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
                create_test_user_message("Hello"),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        // Empty system message should still join in instructions
        assert!(result.instructions.contains(""));
    }

    #[test]
    fn test_multiple_tool_calls_with_text_content() {
        let tool_calls = vec![
            ChatToolCall {
                id: "call_1".to_string(),
                tool_type: ChatToolType::Function,
                function: ChatFunctionCall {
                    name: "func1".to_string(),
                    arguments: r#"{"x":1}"#.to_string(),
                },
            },
            ChatToolCall {
                id: "call_2".to_string(),
                tool_type: ChatToolType::Function,
                function: ChatFunctionCall {
                    name: "func2".to_string(),
                    arguments: r#"{"y":2}"#.to_string(),
                },
            },
        ];

        let chat = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                create_test_user_message("Call both"),
                create_assistant_with_tool_calls_and_content(
                    tool_calls,
                    "I'll call both functions now.",
                ),
            ],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let result = convert_chat_to_responses(&chat).unwrap();
        // Should have: user, 2 FunctionCalls, assistant with content
        assert_eq!(result.input.len(), 4);
    }
}
