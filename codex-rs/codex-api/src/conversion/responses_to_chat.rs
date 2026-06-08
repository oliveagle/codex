//! Conversion from Codex Responses API to OpenAI Chat Completions API.
//!
//! This module provides functionality to convert Codex Responses API
//! responses into OpenAI-compatible Chat Completions response format.

use crate::types::chat::{
    ChatChoice, ChatCompletionResponse, ChatFunctionCall, ChatMessage, ChatMessageRole,
    ChatToolCall, ChatToolType, ChatUsage, FinishReason,
};
use codex_protocol::models::{ContentItem, ResponseItem};
use codex_protocol::protocol::TokenUsage;
use std::time::SystemTime;

/// Converts a Responses API response to a Chat Completions API response.
///
/// # Arguments
///
/// * `id` - The response identifier (e.g., from `response.id`)
/// * `model` - The model name used for the response
/// * `created_at` - Unix timestamp when the response was created (seconds since epoch)
/// * `output` - The output items from the response
/// * `status` - The response status (e.g., "completed", "incomplete", "failed")
/// * `usage` - Optional token usage information
///
/// # Returns
///
/// A `ChatCompletionResponse` compatible with the OpenAI Chat Completions API
///
/// # Conversion Rules
///
/// - **id**: Prefix with "chatcmpl-" if not already prefixed
/// - **object**: Always "chat.completion"
/// - **created**: Use the provided `created_at` timestamp, or current time if 0
/// - **model**: Pass through from the `model` parameter
/// - **choices[0].message**:
///   - role: Always "assistant"
///   - content: Concatenated text from all `OutputText` content items
///   - tool_calls: Converted from `ResponseItem::FunctionCall` items
/// - **choices[0].finish_reason**:
///   - If there are FunctionCall items → "tool_calls"
///   - "completed" → "stop" (if no tool_calls)
///   - "incomplete" → "length"
///   - "failed" → "stop"
///   - default → "stop"
/// - **usage**:
///   - Map from `TokenUsage` if present
///   - `input_tokens` → `prompt_tokens`
///   - `output_tokens` → `completion_tokens`
///   - `total_tokens` → `total_tokens`
///   - None if no usage provided
///
/// # Edge Cases
///
/// - Empty output (e.g., only tool calls) → content = ""
/// - No usage → usage = None
/// - Multiple text items → concatenated
/// - Multiple tool calls → all included in tool_calls array
/// - Non-text content items are ignored for the message content
pub fn convert_responses_to_chat(
    id: &str,
    model: &str,
    created_at: u64,
    output: &[ResponseItem],
    status: &str,
    usage: Option<&TokenUsage>,
) -> ChatCompletionResponse {
    // Prefix ID with "chatcmpl-" if not already
    let chat_id = if id.starts_with("chatcmpl-") {
        id.to_string()
    } else {
        format!("chatcmpl-{}", id)
    };

    // Use created_at, or current time if 0
    let created = if created_at > 0 {
        created_at
    } else {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    };

    // Extract and concatenate text content from output items
    let content = extract_text_content(output);

    // Extract tool calls from FunctionCall items
    let tool_calls = extract_tool_calls(output);

    // Convert status to finish_reason
    // If there are tool_calls, prefer "tool_calls" over the status-based reason
    let finish_reason = if !tool_calls.is_empty() {
        FinishReason::ToolCalls
    } else {
        map_status_to_finish_reason(status)
    };

    // Convert usage if present
    let chat_usage = usage.map(map_token_usage);

    ChatCompletionResponse {
        id: chat_id,
        object: "chat.completion".to_string(),
        created,
        model: model.to_string(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: ChatMessageRole::Assistant,
                content: Some(content),
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                },
                tool_call_id: None,
                name: None,
            },
            finish_reason,
            logprobs: None,
        }],
        usage: chat_usage,
        service_tier: None,
        system_fingerprint: None,
    }
}

/// Extracts and concatenates text content from ResponseItems.
///
/// Only `OutputText` content items are included. Other content types
/// (images, encrypted content, etc.) are ignored.
fn extract_text_content(output: &[ResponseItem]) -> String {
    let mut text_parts = Vec::new();

    for item in output {
        if let ResponseItem::Message { content, .. } = item {
            for content_item in content {
                if let ContentItem::OutputText { text } = content_item {
                    if !text.is_empty() {
                        text_parts.push(text.as_str());
                    }
                }
            }
        }
    }

    text_parts.join("")
}

/// Extracts tool calls from `ResponseItem::FunctionCall` items.
///
/// Each `FunctionCall` is converted to a `ChatToolCall` with the
/// proper id, type, and function name/arguments. Multiple function
/// calls produce multiple tool_calls in the resulting array.
fn extract_tool_calls(output: &[ResponseItem]) -> Vec<ChatToolCall> {
    let mut tool_calls = Vec::new();

    for item in output {
        if let ResponseItem::FunctionCall {
            name,
            arguments,
            call_id,
            ..
        } = item
        {
            tool_calls.push(ChatToolCall {
                id: call_id.clone(),
                tool_type: ChatToolType::Function,
                function: ChatFunctionCall {
                    name: name.clone(),
                    arguments: arguments.clone(),
                },
            });
        }
    }

    tool_calls
}

/// Maps Responses API status to Chat Completions finish_reason.
fn map_status_to_finish_reason(status: &str) -> FinishReason {
    match status {
        "completed" => FinishReason::Stop,
        "incomplete" => FinishReason::Length,
        "failed" => FinishReason::Stop,
        _ => FinishReason::Stop,
    }
}

/// Maps TokenUsage to ChatUsage.
fn map_token_usage(usage: &TokenUsage) -> ChatUsage {
    ChatUsage {
        prompt_tokens: usage.input_tokens as u32,
        completion_tokens: usage.output_tokens as u32,
        total_tokens: usage.total_tokens as u32,
        prompt_tokens_details: None,
        completion_tokens_details: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::protocol::TokenUsage;

    fn create_text_item(text: &str) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: text.to_string(),
            }],
            phase: None,
        }
    }

    #[test]
    fn test_simple_text_response() {
        let output = vec![create_text_item("Hello, world!")];
        let result = convert_responses_to_chat(
            "resp-123",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert_eq!(result.id, "chatcmpl-resp-123");
        assert_eq!(result.object, "chat.completion");
        assert_eq!(result.created, 1677652288);
        assert_eq!(result.model, "gpt-4");
        assert_eq!(result.choices.len(), 1);
        assert_eq!(result.choices[0].index, 0);
        assert_eq!(
            result.choices[0].message.content,
            Some("Hello, world!".to_string())
        );
        assert!(matches!(
            result.choices[0].finish_reason,
            FinishReason::Stop
        ));
        assert!(result.usage.is_none());
    }

    #[test]
    fn test_response_with_usage() {
        let output = vec![create_text_item("Response text")];
        let usage = TokenUsage {
            input_tokens: 10,
            cached_input_tokens: 2,
            output_tokens: 5,
            reasoning_output_tokens: 0,
            total_tokens: 15,
        };

        let result = convert_responses_to_chat(
            "resp-456",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            Some(&usage),
        );

        assert_eq!(result.choices[0].message.content, Some("Response text".to_string()));

        let chat_usage = result.usage.expect("usage should be present");
        assert_eq!(chat_usage.prompt_tokens, 10);
        assert_eq!(chat_usage.completion_tokens, 5);
        assert_eq!(chat_usage.total_tokens, 15);
    }

    #[test]
    fn test_response_without_usage() {
        let output = vec![create_text_item("No usage info")];
        let result = convert_responses_to_chat(
            "resp-789",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert!(result.usage.is_none());
    }

    #[test]
    fn test_empty_content() {
        let output = vec![];
        let result = convert_responses_to_chat(
            "resp-empty",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert_eq!(result.choices[0].message.content, Some("".to_string()));
    }

    #[test]
    fn test_multiple_text_items_concatenated() {
        let output = vec![
            create_text_item("Hello, "),
            create_text_item("world!"),
            create_text_item(" How are you?"),
        ];

        let result = convert_responses_to_chat(
            "resp-multi",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert_eq!(
            result.choices[0].message.content,
            Some("Hello, world! How are you?".to_string())
        );
    }

    #[test]
    fn test_finish_reason_completed_maps_to_stop() {
        let output = vec![create_text_item("Done")];
        let result = convert_responses_to_chat(
            "resp-stop",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert!(matches!(
            result.choices[0].finish_reason,
            FinishReason::Stop
        ));
    }

    #[test]
    fn test_finish_reason_incomplete_maps_to_length() {
        let output = vec![create_text_item("Incomplete")];
        let result = convert_responses_to_chat(
            "resp-length",
            "gpt-4",
            1677652288,
            &output,
            "incomplete",
            None,
        );

        assert!(matches!(
            result.choices[0].finish_reason,
            FinishReason::Length
        ));
    }

    #[test]
    fn test_id_prefixed_correctly() {
        let output = vec![create_text_item("Test")];

        // ID without prefix should get one
        let result1 = convert_responses_to_chat(
            "resp-123",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );
        assert_eq!(result1.id, "chatcmpl-resp-123");

        // ID with existing prefix should stay as-is
        let result2 = convert_responses_to_chat(
            "chatcmpl-abc",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );
        assert_eq!(result2.id, "chatcmpl-abc");
    }

    #[test]
    fn test_zero_created_at_uses_current_time() {
        let output = vec![create_text_item("Test")];
        let before = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let result = convert_responses_to_chat("resp-time", "gpt-4", 0, &output, "completed", None);

        let after = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        assert!(result.created >= before && result.created <= after);
    }

    #[test]
    fn test_non_text_content_items_ignored() {
        let output = vec![ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![
                ContentItem::OutputText {
                    text: "Keep this".to_string(),
                },
                // Images and other content types are ignored
                ContentItem::InputImage {
                    image_url: "data:image/png;base64,ABC".to_string(),
                    detail: None,
                },
            ],
            phase: None,
        }];

        let result = convert_responses_to_chat(
            "resp-mixed",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        // Only text should be included
        assert_eq!(result.choices[0].message.content, Some("Keep this".to_string()));
    }

    #[test]
    fn test_failed_status_maps_to_stop() {
        let output = vec![create_text_item("Failed")];
        let result = convert_responses_to_chat(
            "resp-failed",
            "gpt-4",
            1677652288,
            &output,
            "failed",
            None,
        );

        assert!(matches!(
            result.choices[0].finish_reason,
            FinishReason::Stop
        ));
    }

    #[test]
    fn test_unknown_status_maps_to_stop() {
        let output = vec![create_text_item("Unknown")];
        let result = convert_responses_to_chat(
            "resp-unknown",
            "gpt-4",
            1677652288,
            &output,
            "some_unknown_status",
            None,
        );

        assert!(matches!(
            result.choices[0].finish_reason,
            FinishReason::Stop
        ));
    }

    #[test]
    fn test_message_with_phase_preserved() {
        let output = vec![ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: "Text with phase".to_string(),
            }],
            phase: None,
        }];

        let result = convert_responses_to_chat(
            "resp-phase",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert_eq!(
            result.choices[0].message.content,
            Some("Text with phase".to_string())
        );
    }

    #[test]
    fn test_single_function_call_produces_tool_calls() {
        let output = vec![ResponseItem::FunctionCall {
            id: None,
            name: "get_weather".to_string(),
            namespace: None,
            arguments: r#"{"location": "SF"}"#.to_string(),
            call_id: "call_123".to_string(),
        }];

        let result = convert_responses_to_chat(
            "resp-tools",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        // Should have tool_calls in the message
        let tool_calls = result.choices[0]
            .message
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_123");
        assert!(matches!(tool_calls[0].tool_type, ChatToolType::Function));
        assert_eq!(tool_calls[0].function.name, "get_weather");
        assert_eq!(tool_calls[0].function.arguments, r#"{"location": "SF"}"#);

        // content should be empty
        assert_eq!(result.choices[0].message.content, Some("".to_string()));

        // finish_reason should be tool_calls
        assert!(matches!(
            result.choices[0].finish_reason,
            FinishReason::ToolCalls
        ));
    }

    #[test]
    fn test_multiple_function_calls_all_included() {
        let output = vec![
            ResponseItem::FunctionCall {
                id: None,
                name: "get_weather".to_string(),
                namespace: None,
                arguments: r#"{"location": "SF"}"#.to_string(),
                call_id: "call_1".to_string(),
            },
            ResponseItem::FunctionCall {
                id: None,
                name: "get_time".to_string(),
                namespace: None,
                arguments: r#"{"timezone": "PST"}"#.to_string(),
                call_id: "call_2".to_string(),
            },
        ];

        let result = convert_responses_to_chat(
            "resp-multi",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        let tool_calls = result.choices[0]
            .message
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls.len(), 2);

        // First tool call
        assert_eq!(tool_calls[0].id, "call_1");
        assert_eq!(tool_calls[0].function.name, "get_weather");
        assert_eq!(tool_calls[0].function.arguments, r#"{"location": "SF"}"#);

        // Second tool call
        assert_eq!(tool_calls[1].id, "call_2");
        assert_eq!(tool_calls[1].function.name, "get_time");
        assert_eq!(tool_calls[1].function.arguments, r#"{"timezone": "PST"}"#);

        // finish_reason should be tool_calls
        assert!(matches!(
            result.choices[0].finish_reason,
            FinishReason::ToolCalls
        ));
    }

    #[test]
    fn test_tool_call_with_text_content() {
        // Response that has both text content and tool_calls
        let output = vec![
            ResponseItem::Message {
                id: None,
                role: "assistant".to_string(),
                content: vec![ContentItem::OutputText {
                    text: "I'll check the weather for you.".to_string(),
                }],
                phase: None,
            },
            ResponseItem::FunctionCall {
                id: None,
                name: "get_weather".to_string(),
                namespace: None,
                arguments: r#"{"city": "Seattle"}"#.to_string(),
                call_id: "call_abc".to_string(),
            },
        ];

        let result = convert_responses_to_chat(
            "resp-mixed",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        // Content should have the text
        assert_eq!(
            result.choices[0].message.content,
            Some("I'll check the weather for you.".to_string())
        );

        // And tool_calls should be present
        let tool_calls = result.choices[0]
            .message
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.name, "get_weather");

        // finish_reason should be tool_calls
        assert!(matches!(
            result.choices[0].finish_reason,
            FinishReason::ToolCalls
        ));
    }

    #[test]
    fn test_tool_calls_finish_reason_overrides_status() {
        // Even if status is "completed", the presence of tool_calls should
        // result in finish_reason="tool_calls"
        let output = vec![ResponseItem::FunctionCall {
            id: None,
            name: "my_tool".to_string(),
            namespace: None,
            arguments: "{}".to_string(),
            call_id: "call_xyz".to_string(),
        }];

        let result = convert_responses_to_chat(
            "resp-override",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert!(matches!(
            result.choices[0].finish_reason,
            FinishReason::ToolCalls
        ));
    }

    #[test]
    fn test_no_tool_calls_returns_none() {
        // When there are no tool_calls in the output, message.tool_calls should be None
        let output = vec![ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: "Just a text response".to_string(),
            }],
            phase: None,
        }];

        let result = convert_responses_to_chat(
            "resp-no-tools",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert!(result.choices[0].message.tool_calls.is_none());
        assert!(matches!(
            result.choices[0].finish_reason,
            FinishReason::Stop
        ));
    }

    #[test]
    fn test_tool_call_serializes_to_openai_format() {
        // Verify the output serializes to the expected OpenAI tool_calls JSON format
        let output = vec![ResponseItem::FunctionCall {
            id: None,
            name: "get_weather".to_string(),
            namespace: None,
            arguments: r#"{"location": "Boston"}"#.to_string(),
            call_id: "call_999".to_string(),
        }];

        let result = convert_responses_to_chat(
            "resp-format",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        let json = serde_json::to_string(&result).unwrap();
        // Should contain all the expected fields
        assert!(json.contains(r#""id":"call_999""#));
        assert!(json.contains(r#""type":"function""#));
        assert!(json.contains(r#""name":"get_weather""#));
        assert!(json.contains(r#""arguments":"{\"location\": \"Boston\"}""#));
        assert!(json.contains(r#""finish_reason":"tool_calls""#));
    }

    #[test]
    fn test_function_call_with_empty_arguments() {
        let output = vec![ResponseItem::FunctionCall {
            id: None,
            name: "empty_func".to_string(),
            namespace: None,
            arguments: r#"{}"#.to_string(),
            call_id: "call_empty".to_string(),
        }];

        let result = convert_responses_to_chat(
            "resp-empty",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        let tool_calls = result.choices[0]
            .message
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls[0].function.arguments, "{}");
    }

    #[test]
    fn test_multiple_text_items_interleaved_with_tool_calls() {
        let output = vec![
            create_text_item("First part"),
            create_text_item("Second part"),
            ResponseItem::FunctionCall {
                id: None,
                name: "my_tool".to_string(),
                namespace: None,
                arguments: r#"{"x":1}"#.to_string(),
                call_id: "call_1".to_string(),
            },
            create_text_item("Third part"),
        ];

        let result = convert_responses_to_chat(
            "resp-interleaved",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        // All text parts should be concatenated
        assert_eq!(
            result.choices[0].message.content,
            Some("First partSecond partThird part".to_string())
        );

        // Tool call should be present
        let tool_calls = result.choices[0]
            .message
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls.len(), 1);
    }

    #[test]
    fn test_unicode_content() {
        let output = vec![ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: "Hello 🌍 世界 こんにちは 🎉".to_string(),
            }],
            phase: None,
        }];

        let result = convert_responses_to_chat(
            "resp-unicode",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert!(result.choices[0].message.content.as_ref().unwrap().contains("🌍"));
        assert!(result.choices[0].message.content.as_ref().unwrap().contains("世界"));
    }

    #[test]
    fn test_multiline_content() {
        let text = "Line 1\nLine 2\nLine 3";
        let output = vec![ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: text.to_string(),
            }],
            phase: None,
        }];

        let result = convert_responses_to_chat(
            "resp-multiline",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert_eq!(result.choices[0].message.content, Some(text.to_string()));
    }

    #[test]
    fn test_very_long_content() {
        let long_text = "A".repeat(5000);
        let output = vec![ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: long_text.clone(),
            }],
            phase: None,
        }];

        let result = convert_responses_to_chat(
            "resp-long",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert_eq!(result.choices[0].message.content.as_ref().unwrap().len(), 5000);
    }

    #[test]
    fn test_empty_text_items_are_ignored() {
        let output = vec![
            ResponseItem::Message {
                id: None,
                role: "assistant".to_string(),
                content: vec![
                    ContentItem::OutputText {
                        text: "Keep".to_string(),
                    },
                    ContentItem::OutputText {
                        text: "".to_string(),
                    },
                    ContentItem::OutputText {
                        text: " this".to_string(),
                    },
                ],
                phase: None,
            },
        ];

        let result = convert_responses_to_chat(
            "resp-skip-empty",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        // Empty text items should be skipped in concatenation
        assert_eq!(result.choices[0].message.content, Some("Keep this".to_string()));
    }

    #[test]
    fn test_function_call_with_namespace() {
        let output = vec![ResponseItem::FunctionCall {
            id: Some("func-id-123".to_string()),
            name: "namespaced_func".to_string(),
            namespace: Some("my.namespace".to_string()),
            arguments: r#"{"arg":"val"}"#.to_string(),
            call_id: "call_with_ns".to_string(),
        }];

        let result = convert_responses_to_chat(
            "resp-namespace",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        let tool_calls = result.choices[0]
            .message
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls[0].function.name, "namespaced_func");
        // Namespace is not used in conversion but should not cause error
    }

    #[test]
    fn test_response_with_message_id_and_phase() {
        let output = vec![ResponseItem::Message {
            id: Some("msg-id-456".to_string()),
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: "Response with metadata".to_string(),
            }],
            phase: Some(codex_protocol::models::MessagePhase::FinalAnswer),
        }];

        let result = convert_responses_to_chat(
            "resp-metadata",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        assert_eq!(
            result.choices[0].message.content,
            Some("Response with metadata".to_string())
        );
        // ID and phase are not used in conversion but should not cause error
    }

    #[test]
    fn test_usage_with_all_token_types() {
        let usage = TokenUsage {
            input_tokens: 100,
            cached_input_tokens: 20,
            output_tokens: 50,
            reasoning_output_tokens: 10,
            total_tokens: 150,
        };

        let output = vec![create_text_item("Test")];

        let result = convert_responses_to_chat(
            "resp-usage-all",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            Some(&usage),
        );

        let chat_usage = result.usage.expect("should have usage");
        assert_eq!(chat_usage.prompt_tokens, 100);
        assert_eq!(chat_usage.completion_tokens, 50);
        assert_eq!(chat_usage.total_tokens, 150);
    }

    #[test]
    fn test_multiple_function_calls_with_text() {
        let output = vec![
            create_text_item("I'll make multiple calls"),
            ResponseItem::FunctionCall {
                id: None,
                name: "func1".to_string(),
                namespace: None,
                arguments: r#"{"a":1}"#.to_string(),
                call_id: "call_multi_1".to_string(),
            },
            ResponseItem::FunctionCall {
                id: None,
                name: "func2".to_string(),
                namespace: None,
                arguments: r#"{"b":2}"#.to_string(),
                call_id: "call_multi_2".to_string(),
            },
            ResponseItem::FunctionCall {
                id: None,
                name: "func3".to_string(),
                namespace: None,
                arguments: r#"{"c":3}"#.to_string(),
                call_id: "call_multi_3".to_string(),
            },
        ];

        let result = convert_responses_to_chat(
            "resp-multi-tools",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        // Content should have the text
        assert!(result.choices[0].message.content.as_ref().unwrap().contains("multiple calls"));

        // All tool calls should be present
        let tool_calls = result.choices[0]
            .message
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls.len(), 3);

        // Verify order is preserved
        assert_eq!(tool_calls[0].id, "call_multi_1");
        assert_eq!(tool_calls[1].id, "call_multi_2");
        assert_eq!(tool_calls[2].id, "call_multi_3");
    }

    #[test]
    fn test_cjk_content() {
        let output = vec![ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: "这是中文内容。これは日本語です。한국어 내용입니다.".to_string(),
            }],
            phase: None,
        }];

        let result = convert_responses_to_chat(
            "resp-cjk",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        let content = result.choices[0].message.content.as_ref().unwrap();
        assert!(content.contains("这是"));
        assert!(content.contains("これは"));
        assert!(content.contains("한국어"));
    }

    #[test]
    fn test_emoji_content() {
        let output = vec![ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: "Here are some emojis: 😀 🎉 🚀 💻 🌟".to_string(),
            }],
            phase: None,
        }];

        let result = convert_responses_to_chat(
            "resp-emoji",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        let content = result.choices[0].message.content.as_ref().unwrap();
        assert!(content.contains("😀"));
        assert!(content.contains("🎉"));
    }

    #[test]
    fn test_content_with_special_characters() {
        let special_text = "Special chars: \t\n\r\\\"'<>;&${}()[]";
        let output = vec![ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: special_text.to_string(),
            }],
            phase: None,
        }];

        let result = convert_responses_to_chat(
            "resp-special",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        // Serialize and deserialize to verify JSON safety
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ChatCompletionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.choices[0].message.content, Some(special_text.to_string()));
    }

    #[test]
    fn test_response_with_zero_created_uses_current_time() {
        let output = vec![create_text_item("Test")];

        let before = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let result = convert_responses_to_chat("resp-zero-time", "gpt-4", 0, &output, "completed", None);

        let after = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        assert!(result.created >= before && result.created <= after);
    }

    #[test]
    fn test_empty_arguments_json() {
        let output = vec![ResponseItem::FunctionCall {
            id: None,
            name: "no_args".to_string(),
            namespace: None,
            arguments: r#"{}"#.to_string(),
            call_id: "call_no_args".to_string(),
        }];

        let result = convert_responses_to_chat(
            "resp-no-args",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        let tool_calls = result.choices[0]
            .message
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls[0].function.arguments, "{}");
    }

    #[test]
    fn test_complex_arguments_json() {
        let complex_args = r#"{"nested":{"object":{"array":[1,2,3],"string":"value"},"number":123.45},"boolean":true,"null":null}"#;
        let output = vec![ResponseItem::FunctionCall {
            id: None,
            name: "complex_func".to_string(),
            namespace: None,
            arguments: complex_args.to_string(),
            call_id: "call_complex".to_string(),
        }];

        let result = convert_responses_to_chat(
            "resp-complex-args",
            "gpt-4",
            1677652288,
            &output,
            "completed",
            None,
        );

        let tool_calls = result.choices[0]
            .message
            .tool_calls
            .as_ref()
            .expect("should have tool_calls");
        assert_eq!(tool_calls[0].function.arguments, complex_args);

        // Verify JSON is valid
        let _: serde_json::Value = serde_json::from_str(&tool_calls[0].function.arguments)
            .expect("arguments should be valid JSON");
    }
}
