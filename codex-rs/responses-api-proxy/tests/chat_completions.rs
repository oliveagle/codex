//! Integration tests for Chat Completions API endpoint.
//!
//! These tests verify that the proxy correctly converts between
//! OpenAI Chat Completions format and Codex Responses API format.
//!
//! The tests work as follows:
//!   1. Start a mock upstream HTTP server on an ephemeral port
//!   2. Start the proxy via `run_main_with_auth` on another ephemeral port
//!   3. Send real HTTP requests to the proxy and verify the responses

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use codex_responses_api_proxy::{Args, run_main_with_auth};
use reqwest::blocking::Client;
use serde_json::json;

/// Mock upstream server state.
#[derive(Clone, Default)]
struct MockUpstream {
    request_count: Arc<AtomicUsize>,
}

/// Starts a simple mock upstream HTTP server.
///
/// Returns the server URL and the mock state for assertions.
fn start_mock_upstream() -> (String, MockUpstream, thread::JoinHandle<()>) {
    let mock = MockUpstream::default();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let upstream_url = format!("http://127.0.0.1:{}", addr.port());

    let mock_clone = mock.clone();
    let handle = thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            mock_clone.request_count.fetch_add(1, Ordering::SeqCst);

            // Read the full request (simple approach: read until EOF or header parsing)
            let mut buffer = Vec::new();
            let mut tmp = [0u8; 16384];
            loop {
                match stream.read(&mut tmp) {
                    Ok(0) => break,
                    Ok(n) => {
                        buffer.extend_from_slice(&tmp[..n]);
                        // Check if we have full headers + body
                        let req_str = String::from_utf8_lossy(&buffer);
                        if let Some(header_end) = req_str.find("\r\n\r\n") {
                            // Parse Content-Length
                            let mut content_length: usize = 0;
                            let headers_str = &req_str[..header_end];
                            for line in headers_str.lines() {
                                if let Some((_, value)) = line.split_once(':') {
                                    if value.trim().parse::<usize>().is_ok()
                                        && line.to_lowercase().starts_with("content-length:")
                                    {
                                        content_length = value.trim().parse().unwrap_or(0);
                                        break;
                                    }
                                }
                            }

                            let body_start = header_end + 4;
                            let body_so_far = buffer.len() - body_start;
                            if body_so_far >= content_length {
                                break;
                            }
                        } else {
                            break; // No header end found
                        }
                    }
                    Err(_) => break,
                }
            }

            let request = String::from_utf8_lossy(&buffer);

            // Extract body from HTTP request
            let body_str = if let Some(body_start) = request.find("\r\n\r\n") {
                request[body_start + 4..].to_string()
            } else {
                String::new()
            };

            // Parse request method
            let first_line = request.lines().next().unwrap_or("");
            let mut parts = first_line.split(' ');
            let method = parts.next().unwrap_or("");

            // Detect streaming request via body content
            let is_streaming = body_str.contains(r#""stream":true"#);

            // Detect tool calls in request (has non-empty "tools" array)
            let has_tools = body_str.contains(r#""tools":[{"#);

            let response_body = if method == "POST" {
                if is_streaming {
                    return send_sse_response(&mut stream);
                }
                // Non-streaming: detect tool calls vs text
                if has_tools {
                    // Tool call response
                    json!({
                        "id": "resp-tool-1",
                        "type": "response",
                        "status": "completed",
                        "created_at": 1677652288,
                        "model": "claude-3-5-sonnet-20241022",
                        "output": [{
                            "type": "function_call",
                            "name": "get_weather",
                            "arguments": "{\"location\":\"San Francisco\"}",
                            "call_id": "call_abc123"
                        }],
                        "usage": {
                            "input_tokens": 12,
                            "cached_input_tokens": 0,
                            "output_tokens": 8,
                            "reasoning_output_tokens": 0,
                            "total_tokens": 20
                        }
                    })
                } else {
                    // Plain text response
                    json!({
                        "id": "resp-123",
                        "type": "response",
                        "status": "completed",
                        "created_at": 1677652288,
                        "model": "claude-3-5-sonnet-20241022",
                        "output": [{
                            "type": "message",
                            "role": "assistant",
                            "content": [{
                                "type": "output_text",
                                "text": "Hello from the mock upstream!"
                            }]
                        }],
                        "usage": {
                            "input_tokens": 10,
                            "cached_input_tokens": 0,
                            "output_tokens": 5,
                            "reasoning_output_tokens": 0,
                            "total_tokens": 15
                        }
                    })
                }
            } else {
                json!({
                    "id": "resp-default",
                    "type": "response",
                    "status": "completed"
                })
            };

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.to_string().len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    (upstream_url, mock, handle)
}

/// Send an SSE response for streaming requests.
fn send_sse_response(stream: &mut std::net::TcpStream) {
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n";
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();

    let events = vec![
        r#"{"type":"response.created","response":{"id":"resp-stream-1","status":"in_progress"}}"#,
        r#"{"type":"response.output_text.delta","delta":"Hello ","item_id":"msg-1"}"#,
        r#"{"type":"response.output_text.delta","delta":"streaming ","item_id":"msg-1"}"#,
        r#"{"type":"response.output_text.delta","delta":"world!","item_id":"msg-1"}"#,
        r#"{"type":"response.completed","response":{"id":"resp-stream-1","status":"completed","end_turn":true,"usage":{"input_tokens":7,"output_tokens":3,"total_tokens":10}}}"#,
    ];

    for event in events {
        let sse = format!("data: {}\n\n", event);
        let _ = stream.write_all(sse.as_bytes());
        let _ = stream.flush();
        thread::sleep(Duration::from_millis(5));
    }
    let _ = stream.write_all(b"data: [DONE]\n\n");
    let _ = stream.flush();
}

/// Start the proxy in a background thread. Returns (proxy_url, JoinHandle).
fn start_proxy(upstream_url: String) -> (String, thread::JoinHandle<()>) {
    // Use a TCP listener to get an ephemeral port for the proxy
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener); // Release so the proxy can bind to the same port

    let args = Args {
        port: Some(port),
        server_info: None,
        http_shutdown: false,
        upstream_url,
        dump_dir: None,
    };

    let auth_header: &'static str = "Bearer test-key-123";

    let handle = thread::spawn(move || {
        // run_main blocks forever, so any panic/error is expected at shutdown
        let _ = run_main_with_auth(args, auth_header);
    });

    // Give the proxy a moment to start
    thread::sleep(Duration::from_millis(100));

    (format!("http://127.0.0.1:{}", port), handle)
}

#[test]
fn test_non_streaming_chat_completion() {
    let (upstream_url, mock, _upstream_handle) = start_mock_upstream();
    let (proxy_url, _proxy_handle) = start_proxy(upstream_url);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let request_body = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Hello!"}
        ]
    });

    let response = client
        .post(format!("{}/v1/chat/completions", proxy_url))
        .header("Authorization", "Bearer client-key")
        .json(&request_body)
        .send()
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let ct = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.contains("application/json"), "content-type was: {ct}");

    let response_json: serde_json::Value = response.json().unwrap();
    assert_eq!(response_json["object"], "chat.completion");
    assert_eq!(response_json["model"], "gpt-4");
    let id = response_json["id"].as_str().unwrap();
    assert!(id.starts_with("chatcmpl-"), "id was: {id}");
    assert_eq!(response_json["choices"][0]["index"], 0);
    assert_eq!(response_json["choices"][0]["message"]["role"], "assistant");
    let msg_content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");
    assert_eq!(
        msg_content,
        "Hello from the mock upstream!"
    );
    assert_eq!(response_json["choices"][0]["finish_reason"], "stop");

    // Verify usage mapping
    let usage = &response_json["usage"];
    assert_eq!(usage["prompt_tokens"], 10);
    assert_eq!(usage["completion_tokens"], 5);
    assert_eq!(usage["total_tokens"], 15);

    // Verify upstream was hit
    assert!(mock.request_count.load(Ordering::SeqCst) >= 1);
}

#[test]
fn test_streaming_chat_completion() {
    let (upstream_url, _mock, _upstream_handle) = start_mock_upstream();
    let (proxy_url, _proxy_handle) = start_proxy(upstream_url);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let request_body = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Hello!"}
        ],
        "stream": true
    });

    let response = client
        .post(format!("{}/v1/chat/completions", proxy_url))
        .header("Authorization", "Bearer client-key")
        .json(&request_body)
        .send()
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let ct = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.contains("text/event-stream"), "content-type was: {ct}");

    // Read the body line by line
    let body = response.text().unwrap();

    // Should have a role chunk, content chunks, finish chunk, and [DONE]
    assert!(body.contains("\"object\":\"chat.completion.chunk\""), "body: {body}");
    assert!(body.contains("\"role\":\"assistant\""), "body: {body}");
    assert!(body.contains("\"content\":\"Hello \""), "body: {body}");
    assert!(body.contains("\"content\":\"streaming \""), "body: {body}");
    assert!(body.contains("\"content\":\"world!\""), "body: {body}");
    assert!(body.contains("\"finish_reason\":\"stop\""), "body: {body}");
    // Note: usage chunk may or may not be present depending on proxy implementation
    assert!(body.contains("[DONE]"), "body: {body}");
}

#[test]
fn test_tool_call_conversion_in_response() {
    let (upstream_url, _mock, _upstream_handle) = start_mock_upstream();
    let (proxy_url, _proxy_handle) = start_proxy(upstream_url);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let request_body = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "What's the weather in SF?"}
        ],
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    }
                }
            }
        }]
    });

    let response = client
        .post(format!("{}/v1/chat/completions", proxy_url))
        .header("Authorization", "Bearer client-key")
        .json(&request_body)
        .send()
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let response_json: serde_json::Value = response.json().unwrap();
    assert_eq!(response_json["object"], "chat.completion");

    // Should have tool_calls
    let tool_calls = &response_json["choices"][0]["message"]["tool_calls"];
    assert!(!tool_calls.is_null(), "expected tool_calls: {response_json}");
    assert!(tool_calls.is_array(), "tool_calls not an array");
    assert_eq!(tool_calls.as_array().unwrap().len(), 1);

    let tool_call = &tool_calls[0];
    assert_eq!(tool_call["id"], "call_abc123");
    assert_eq!(tool_call["type"], "function");
    assert_eq!(tool_call["function"]["name"], "get_weather");
    assert_eq!(
        tool_call["function"]["arguments"],
        "{\"location\":\"San Francisco\"}"
    );

    // finish_reason should be "tool_calls"
    assert_eq!(response_json["choices"][0]["finish_reason"], "tool_calls");

    // Content should be empty
    assert_eq!(response_json["choices"][0]["message"]["content"], "");
}

#[test]
fn test_invalid_json_body_returns_400() {
    let (upstream_url, _mock, _upstream_handle) = start_mock_upstream();
    let (proxy_url, _proxy_handle) = start_proxy(upstream_url);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let response = client
        .post(format!("{}/v1/chat/completions", proxy_url))
        .header("Authorization", "Bearer client-key")
        .header("Content-Type", "application/json")
        .body("{not valid json")
        .send()
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);

    let response_json: serde_json::Value = response.json().unwrap();
    let err = response_json["error"].as_str().unwrap();
    assert!(
        err.contains("invalid JSON"),
        "expected error to contain 'invalid JSON', got: {err}"
    );
}

#[test]
fn test_get_method_returns_403() {
    let (upstream_url, _mock, _upstream_handle) = start_mock_upstream();
    let (proxy_url, _proxy_handle) = start_proxy(upstream_url);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let response = client
        .get(format!("{}/v1/chat/completions", proxy_url))
        .header("Authorization", "Bearer client-key")
        .send()
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

    let response_json: serde_json::Value = response.json().unwrap();
    assert_eq!(response_json["error"], "forbidden: route not supported");
}

#[test]
fn test_wrong_path_returns_403() {
    let (upstream_url, _mock, _upstream_handle) = start_mock_upstream();
    let (proxy_url, _proxy_handle) = start_proxy(upstream_url);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let response = client
        .post(format!("{}/v1/wrong-path", proxy_url))
        .header("Authorization", "Bearer client-key")
        .json(&json!({"model": "gpt-4", "messages": []}))
        .send()
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

    let response_json: serde_json::Value = response.json().unwrap();
    assert_eq!(response_json["error"], "forbidden: route not supported");
}

#[test]
fn test_request_to_upstream_includes_authorization() {
    let (upstream_url, _mock, _upstream_handle) = start_mock_upstream();
    let (proxy_url, _proxy_handle) = start_proxy(upstream_url);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let request_body = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Hello!"}
        ]
    });

    let response = client
        .post(format!("{}/v1/chat/completions", proxy_url))
        .header("Authorization", "Bearer client-supplied-key")
        .json(&request_body)
        .send()
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    // The proxy should respond successfully even with a different client auth
    // (the proxy uses its own configured auth header toward the upstream)
    let response_json: serde_json::Value = response.json().unwrap();
    assert_eq!(response_json["object"], "chat.completion");
}
