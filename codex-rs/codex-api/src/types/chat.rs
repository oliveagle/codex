//! OpenAI Chat Completions API data structures.
//!
//! This module defines types compatible with the OpenAI Chat Completions API specification.
//! These types are used for requests and responses in chat completion operations.

use serde::{Deserialize, Serialize};

/// Request payload for creating a chat completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// ID of the model to use.
    pub model: String,

    /// Messages in the conversation so far.
    pub messages: Vec<ChatMessage>,

    /// Sampling temperature to use, between 0 and 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Nucleus sampling threshold.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Whether to stream partial message deltas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Deprecated: Use max_completion_tokens instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Maximum number of tokens that can be generated in the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,

    /// Tools the model may call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatTool>>,

    /// Controls which tool to call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ChatToolChoice>,

    /// Sequences where the API will stop generating further tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<ChatStop>,
}

/// A message in the chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message author.
    pub role: ChatMessageRole,

    /// The content of the message (for roles that have text content).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Tool calls made by the assistant (for assistant messages with tool calls).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatToolCall>>,

    /// Tool call ID (for tool response messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// The name of the assistant (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// The role of a message author.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatMessageRole {
    /// System message providing instructions.
    System,

    /// User message.
    User,

    /// Assistant message.
    Assistant,

    /// Tool response message.
    Tool,
}

/// Content that can be in a chat message.
///
/// This is a convenience type for handling different message content formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatMessageContent {
    /// Simple text content.
    Text(String),

    /// Structured content with role.
    Structured {
        role: ChatMessageRole,
        content: String,
    },
}

/// A tool definition that the model can call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTool {
    /// The type of tool. Currently only "function" is supported.
    #[serde(rename = "type")]
    pub tool_type: ChatToolType,

    /// The function definition.
    pub function: ChatFunction,
}

/// The type of tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatToolType {
    /// Function tool.
    Function,
}

/// Function definition for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFunction {
    /// The name of the function to call.
    pub name: String,

    /// Description of what the function does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// JSON Schema for the function parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,

    /// Whether to enable strict schema validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Controls which tool to call, if any.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatToolChoice {
    /// Model can choose between generating a message or calling a tool.
    Auto,

    /// Model will not call any tools.
    None,

    /// Model must call the specified function.
    #[serde(rename = "required")]
    Required(ChatRequiredFunction),

    /// Specific function to call.
    #[serde(rename = "type")]
    Specific {
        #[serde(rename = "type")]
        tool_type: ChatToolType,
        function: ChatFunctionChoice,
    },
}

/// Required function configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequiredFunction {
    /// Function definition for the required call.
    pub function: ChatFunctionChoice,
}

/// Specific function to call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFunctionChoice {
    /// The name of the function to call.
    pub name: String,
}

/// Stop sequences for completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatStop {
    /// Single stop string.
    Single(String),

    /// Multiple stop strings.
    Multiple(Vec<String>),
}

/// Non-streaming chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// Unique identifier for the completion.
    pub id: String,

    /// Object type, always "chat.completion".
    pub object: String,

    /// Unix timestamp (in seconds) of when the completion was created.
    pub created: u64,

    /// The model used for the completion.
    pub model: String,

    /// The completion choices.
    pub choices: Vec<ChatChoice>,

    /// Token usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ChatUsage>,

    /// Service tier information (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,

    /// The reason the completion finished.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// A completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    /// Index of the choice in the list.
    pub index: u32,

    /// The message generated by the model.
    pub message: ChatMessage,

    /// The reason the model stopped generating.
    pub finish_reason: FinishReason,

    /// Log probabilities (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// Reason why the model stopped generating.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Model hit a natural stop point or provided stop sequence.
    Stop,

    /// Model requested a tool call.
    ToolCalls,

    /// Model hit max token limit.
    Length,

    /// Content was omitted due to a flag.
    ContentFilter,
}

/// Streaming chunk for chat completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier for the completion.
    pub id: String,

    /// Object type, always "chat.completion.chunk".
    pub object: String,

    /// Unix timestamp (in seconds) of when the chunk was created.
    pub created: u64,

    /// The model used for the completion.
    pub model: String,

    /// The completion choices for this chunk.
    pub choices: Vec<ChatChunkChoice>,

    /// Token usage information (included in final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ChatUsage>,

    /// Service tier information (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,

    /// System fingerprint (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// A choice in a streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunkChoice {
    /// Index of the choice.
    pub index: u32,

    /// The delta message update.
    pub delta: ChatMessageDelta,

    /// The reason the model stopped (only in final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,

    /// Log probabilities (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// Delta message update for streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDelta {
    /// The role of the author (only in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<ChatMessageRole>,

    /// The content delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Tool call deltas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatToolCallDelta>>,

    /// Context for tool calls (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

/// A tool call made by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatToolCall {
    /// Unique ID for the tool call.
    pub id: String,

    /// The type of tool. Currently only "function" is supported.
    #[serde(rename = "type")]
    pub tool_type: ChatToolType,

    /// The function call details.
    pub function: ChatFunctionCall,
}

/// Function call details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFunctionCall {
    /// The name of the function to call.
    pub name: String,

    /// JSON string of arguments to pass to the function.
    pub arguments: String,
}

/// Delta for a tool call during streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatToolCallDelta {
    /// Index of the tool call in the tool_calls array.
    pub index: u32,

    /// Unique ID for the tool call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// The type of tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub tool_type: Option<ChatToolType>,

    /// The function call delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ChatFunctionCallDelta>,
}

/// Delta for function call details during streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFunctionCallDelta {
    /// The name of the function to call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// JSON string delta of arguments to pass to the function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// Token usage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatUsage {
    /// Number of tokens in the prompt.
    pub prompt_tokens: u32,

    /// Number of tokens in the completion.
    pub completion_tokens: u32,

    /// Total number of tokens (prompt + completion).
    pub total_tokens: u32,

    /// Number of tokens in reasoning (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<serde_json::Value>,

    /// Number of tokens in completion details (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_chat_message() {
        let msg = ChatMessage {
            role: ChatMessageRole::User,
            content: Some("Hello, world!".to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"role":"user","content":"Hello, world!"}"#);
    }

    #[test]
    fn test_deserialize_chat_message() {
        let json = r#"{"role":"user","content":"Hello, world!"}"#;
        let msg: ChatMessage = serde_json::from_str(json).unwrap();

        assert!(matches!(msg.role, ChatMessageRole::User));
        assert_eq!(msg.content, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_serialize_chat_completion_request() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatMessageRole::User,
                content: Some("Hello!".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            temperature: Some(0.7),
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""model":"gpt-4""#));
        assert!(json.contains(r#""temperature":0.7"#));
    }

    #[test]
    fn test_deserialize_chat_completion_request() {
        let json = r#"{
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello!"}
            ],
            "temperature": 0.7
        }"#;

        let request: ChatCompletionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_serialize_chat_message_with_tool_calls() {
        let msg = ChatMessage {
            role: ChatMessageRole::Assistant,
            content: None,
            tool_calls: Some(vec![ChatToolCall {
                id: "call_123".to_string(),
                tool_type: ChatToolType::Function,
                function: ChatFunctionCall {
                    name: "get_weather".to_string(),
                    arguments: r#"{"location": "NYC"}"#.to_string(),
                },
            }]),
            tool_call_id: None,
            name: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""role":"assistant""#));
        assert!(json.contains(r#""tool_calls""#));
    }

    #[test]
    fn test_deserialize_tool_message() {
        let json = r#"{
            "role": "tool",
            "tool_call_id": "call_123",
            "content": "The weather is 75°F"
        }"#;

        let msg: ChatMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg.role, ChatMessageRole::Tool));
        assert_eq!(msg.tool_call_id, Some("call_123".to_string()));
        assert_eq!(msg.content, Some("The weather is 75°F".to_string()));
    }

    #[test]
    fn test_serialize_chat_stop_single() {
        let stop = ChatStop::Single("END".to_string());
        let json = serde_json::to_string(&stop).unwrap();
        assert_eq!(json, r#""END""#);
    }

    #[test]
    fn test_serialize_chat_stop_multiple() {
        let stop = ChatStop::Multiple(vec!["END".to_string(), "STOP".to_string()]);
        let json = serde_json::to_string(&stop).unwrap();
        assert_eq!(json, r#"["END","STOP"]"#);
    }

    #[test]
    fn test_deserialize_chat_stop_single() {
        let json = r#""END""#;
        let stop: ChatStop = serde_json::from_str(json).unwrap();
        assert!(matches!(stop, ChatStop::Single(_)));
    }

    #[test]
    fn test_deserialize_chat_stop_multiple() {
        let json = r#"["END","STOP"]"#;
        let stop: ChatStop = serde_json::from_str(json).unwrap();
        assert!(matches!(stop, ChatStop::Multiple(_)));
    }

    #[test]
    fn test_serialize_chat_completion_response() {
        let response = ChatCompletionResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1677652288,
            model: "gpt-4".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: ChatMessageRole::Assistant,
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
                finish_reason: FinishReason::Stop,
                logprobs: None,
            }],
            usage: Some(ChatUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                prompt_tokens_details: None,
                completion_tokens_details: None,
            }),
            service_tier: None,
            system_fingerprint: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""id":"chatcmpl-123""#));
        assert!(json.contains(r#""object":"chat.completion""#));
        assert!(json.contains(r#""finish_reason":"stop""#));
    }

    #[test]
    fn test_serialize_chat_completion_chunk() {
        let chunk = ChatCompletionChunk {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1677652288,
            model: "gpt-4".to_string(),
            choices: vec![ChatChunkChoice {
                index: 0,
                delta: ChatMessageDelta {
                    role: Some(ChatMessageRole::Assistant),
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                    context: None,
                },
                finish_reason: None,
                logprobs: None,
            }],
            usage: None,
            service_tier: None,
            system_fingerprint: None,
        };

        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains(r#""object":"chat.completion.chunk""#));
        assert!(json.contains(r#""delta":{"role":"assistant","content":"Hello"}"#));
    }

    #[test]
    fn test_serialize_chat_tool() {
        let tool = ChatTool {
            tool_type: ChatToolType::Function,
            function: ChatFunction {
                name: "get_weather".to_string(),
                description: Some("Get current weather".to_string()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    }
                })),
                strict: None,
            },
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains(r#""type":"function""#));
        assert!(json.contains(r#""name":"get_weather""#));
    }

    #[test]
    fn test_serialize_chat_tool_choice() {
        let auto = ChatToolChoice::Auto;
        let json = serde_json::to_string(&auto).unwrap();
        assert_eq!(json, r#""auto""#);

        let none = ChatToolChoice::None;
        let json = serde_json::to_string(&none).unwrap();
        assert_eq!(json, r#""none""#);
    }

    #[test]
    fn test_deserialize_finish_reason() {
        let json = r#""tool_calls""#;
        let reason: FinishReason = serde_json::from_str(json).unwrap();
        assert!(matches!(reason, FinishReason::ToolCalls));
    }

    #[test]
    fn test_serialize_finish_reason_length() {
        let reason = FinishReason::Length;
        let json = serde_json::to_string(&reason).unwrap();
        assert_eq!(json, r#""length""#);
    }

    #[test]
    fn test_deserialize_finish_reason_length() {
        let json = r#""length""#;
        let reason: FinishReason = serde_json::from_str(json).unwrap();
        assert!(matches!(reason, FinishReason::Length));
    }

    #[test]
    fn test_serialize_finish_reason_content_filter() {
        let reason = FinishReason::ContentFilter;
        let json = serde_json::to_string(&reason).unwrap();
        assert_eq!(json, r#""content_filter""#);
    }

    #[test]
    fn test_deserialize_finish_reason_content_filter() {
        let json = r#""content_filter""#;
        let reason: FinishReason = serde_json::from_str(json).unwrap();
        assert!(matches!(reason, FinishReason::ContentFilter));
    }

    #[test]
    fn test_serialize_chat_message_with_empty_content() {
        let msg = ChatMessage {
            role: ChatMessageRole::User,
            content: Some("".to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"role":"user","content":""}"#);
    }

    #[test]
    fn test_serialize_chat_message_with_system_role() {
        let msg = ChatMessage {
            role: ChatMessageRole::System,
            content: Some("You are helpful".to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"role":"system","content":"You are helpful"}"#);
    }

    #[test]
    fn test_deserialize_chat_message_with_assistant_role() {
        let json = r#"{"role":"assistant","content":"Hello!"}"#;
        let msg: ChatMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg.role, ChatMessageRole::Assistant));
    }

    #[test]
    fn test_serialize_chat_message_with_unicode_content() {
        let msg = ChatMessage {
            role: ChatMessageRole::User,
            content: Some("Hello 🌍 世界 こんにちは".to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("🌍"));
        assert!(json.contains("世界"));
    }

    #[test]
    fn test_serialize_chat_message_with_multiline_content() {
        let msg = ChatMessage {
            role: ChatMessageRole::User,
            content: Some("Line 1\nLine 2\nLine 3".to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Line 1\\nLine 2"));
    }

    #[test]
    fn test_serialize_chat_completion_chunk_with_usage() {
        let chunk = ChatCompletionChunk {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 1677652288,
            model: "gpt-4".to_string(),
            choices: vec![ChatChunkChoice {
                index: 0,
                delta: ChatMessageDelta {
                    role: None,
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                    context: None,
                },
                finish_reason: Some(FinishReason::Stop),
                logprobs: None,
            }],
            usage: Some(ChatUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                prompt_tokens_details: None,
                completion_tokens_details: None,
            }),
            service_tier: None,
            system_fingerprint: None,
        };

        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains(r#""usage""#));
        assert!(json.contains(r#""prompt_tokens":10"#));
    }

    #[test]
    fn test_serialize_chat_usage_with_details() {
        let usage = ChatUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            prompt_tokens_details: Some(serde_json::json!({"cached_tokens": 2})),
            completion_tokens_details: Some(serde_json::json!({"reasoning_tokens": 1})),
        };

        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains(r#""prompt_tokens_details""#));
        assert!(json.contains(r#""cached_tokens":2"#));
    }

    #[test]
    fn test_serialize_chat_message_delta_with_tool_calls() {
        let delta = ChatMessageDelta {
            role: Some(ChatMessageRole::Assistant),
            content: None,
            tool_calls: Some(vec![ChatToolCallDelta {
                index: 0,
                id: Some("call_123".to_string()),
                tool_type: Some(ChatToolType::Function),
                function: Some(ChatFunctionCallDelta {
                    name: Some("get_weather".to_string()),
                    arguments: Some(r#"{"city":"NYC"}"#.to_string()),
                }),
            }]),
            context: None,
        };

        let json = serde_json::to_string(&delta).unwrap();
        assert!(json.contains(r#""tool_calls""#));
        assert!(json.contains(r#""index":0"#));
        assert!(json.contains(r#""id":"call_123""#));
    }

    #[test]
    fn test_serialize_chat_tool_call_delta_with_only_id() {
        let delta = ChatToolCallDelta {
            index: 0,
            id: Some("call_456".to_string()),
            tool_type: None,
            function: None,
        };

        let json = serde_json::to_string(&delta).unwrap();
        assert!(json.contains(r#""id":"call_456""#));
        assert!(!json.contains(r#""type""#));
        assert!(!json.contains(r#""function""#));
    }

    #[test]
    fn test_serialize_chat_tool_call_delta_with_only_arguments() {
        let delta = ChatToolCallDelta {
            index: 1,
            id: None,
            tool_type: None,
            function: Some(ChatFunctionCallDelta {
                name: None,
                arguments: Some(r#"{"arg":"value"}"#.to_string()),
            }),
        };

        let json = serde_json::to_string(&delta).unwrap();
        assert!(json.contains(r#""arguments""#));
        assert!(!json.contains(r#""id""#));
    }

    #[test]
    fn test_serialize_chat_function_call_delta_with_only_name() {
        let delta = ChatFunctionCallDelta {
            name: Some("my_function".to_string()),
            arguments: None,
        };

        let json = serde_json::to_string(&delta).unwrap();
        assert!(json.contains(r#""name":"my_function""#));
        assert!(!json.contains(r#""arguments""#));
    }

    #[test]
    fn test_serialize_chat_tool_choice_required() {
        use crate::types::chat::{ChatFunctionChoice, ChatRequiredFunction};

        let required = ChatToolChoice::Required(ChatRequiredFunction {
            function: ChatFunctionChoice {
                name: "my_func".to_string(),
            },
        });

        let json = serde_json::to_string(&required).unwrap();
        // Required uses serde rename to "required"
        assert!(json.contains(r#""required""#));
        assert!(json.contains(r#""name":"my_func""#));
    }

    #[test]
    fn test_serialize_chat_tool_choice_specific() {
        use crate::types::chat::{ChatFunctionChoice};

        let specific = ChatToolChoice::Specific {
            tool_type: ChatToolType::Function,
            function: ChatFunctionChoice {
                name: "specific_func".to_string(),
            },
        };

        let json = serde_json::to_string(&specific).unwrap();
        assert!(json.contains(r#""type":"function""#));
        assert!(json.contains(r#""name":"specific_func""#));
    }

    #[test]
    fn test_serialize_chat_function_without_parameters() {
        let func = ChatFunction {
            name: "simple_func".to_string(),
            description: Some("A simple function".to_string()),
            parameters: None,
            strict: None,
        };

        let json = serde_json::to_string(&func).unwrap();
        assert!(json.contains(r#""name":"simple_func""#));
        assert!(!json.contains(r#""parameters""#));
    }

    #[test]
    fn test_serialize_chat_function_with_strict() {
        let func = ChatFunction {
            name: "strict_func".to_string(),
            description: None,
            parameters: Some(serde_json::json!({"type": "object"})),
            strict: Some(true),
        };

        let json = serde_json::to_string(&func).unwrap();
        assert!(json.contains(r#""strict":true"#));
    }

    #[test]
    fn test_serialize_chat_completion_request_with_stream() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatMessageRole::User,
                content: Some("Hello!".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            temperature: None,
            top_p: None,
            stream: Some(true),
            max_tokens: None,
            max_completion_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""stream":true"#));
    }

    #[test]
    fn test_serialize_chat_completion_request_with_tools() {
        use crate::types::chat::ChatTool;

        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatMessageRole::User,
                content: Some("What's the weather?".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            max_completion_tokens: None,
            tools: Some(vec![ChatTool {
                tool_type: ChatToolType::Function,
                function: ChatFunction {
                    name: "get_weather".to_string(),
                    description: Some("Get weather".to_string()),
                    parameters: Some(serde_json::json!({"type": "object"})),
                    strict: None,
                },
            }]),
            tool_choice: None,
            stop: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""tools""#));
        assert!(json.contains(r#""name":"get_weather""#));
    }

    #[test]
    fn test_serialize_chat_completion_response_with_system_fingerprint() {
        let response = ChatCompletionResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1677652288,
            model: "gpt-4".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: ChatMessageRole::Assistant,
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
                finish_reason: FinishReason::Stop,
                logprobs: None,
            }],
            usage: None,
            service_tier: Some("default".to_string()),
            system_fingerprint: Some("fp_123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""service_tier":"default""#));
        assert!(json.contains(r#""system_fingerprint":"fp_123""#));
    }

    #[test]
    fn test_deserialize_chat_message_with_all_fields() {
        let json = r#"{
            "role": "assistant",
            "content": "Hello!",
            "tool_calls": [{
                "id": "call_123",
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "arguments": "{\"city\":\"NYC\"}"
                }
            }],
            "name": "Assistant"
        }"#;

        let msg: ChatMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg.role, ChatMessageRole::Assistant));
        assert!(msg.tool_calls.is_some());
        assert_eq!(msg.name, Some("Assistant".to_string()));
    }
}
