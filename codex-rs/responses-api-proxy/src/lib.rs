use std::fs::File;
use std::fs::{self};
use std::io::BufRead;
use std::io::BufReader;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use clap::Parser;
use reqwest::Url;
use reqwest::blocking::Client;
use reqwest::header::AUTHORIZATION;
use reqwest::header::HOST;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderName;
use reqwest::header::HeaderValue;
use serde::Deserialize;
use serde::Serialize;
use tiny_http::Header;
use tiny_http::Method;
use tiny_http::Request;
use tiny_http::Response;
use tiny_http::Server;
use tiny_http::StatusCode;

mod dump;
mod read_api_key;
use dump::ExchangeDump;
use dump::ExchangeDumper;
use read_api_key::read_auth_header_from_stdin;

pub use codex_api::conversion::chat_to_responses::convert_chat_to_responses;
pub use codex_api::conversion::responses_to_chat::convert_responses_to_chat;
pub use codex_api::types::chat::ChatCompletionRequest;

/// CLI arguments for the proxy.
#[derive(Debug, Clone, Parser)]
#[command(name = "responses-api-proxy", about = "Minimal OpenAI responses proxy")]
pub struct Args {
    /// Port to listen on. If not set, an ephemeral port is used.
    #[arg(long)]
    pub port: Option<u16>,

    /// Path to a JSON file to write startup info (single line). Includes {"port": <u16>}.
    #[arg(long, value_name = "FILE")]
    pub server_info: Option<PathBuf>,

    /// Enable HTTP shutdown endpoint at GET /shutdown
    #[arg(long)]
    pub http_shutdown: bool,

    /// Absolute URL the proxy should forward requests to (defaults to OpenAI).
    #[arg(long, default_value = "https://api.openai.com/v1/responses")]
    pub upstream_url: String,

    /// Directory where request/response dumps should be written as JSON.
    #[arg(long, value_name = "DIR")]
    pub dump_dir: Option<PathBuf>,
}

#[derive(Serialize)]
struct ServerInfo {
    port: u16,
    pid: u32,
}

/// Configuration for forwarding requests to upstream.
pub struct ForwardConfig {
    pub upstream_url: Url,
    pub host_header: HeaderValue,
}

/// Entry point for the library main, for parity with other crates.
pub fn run_main(args: Args) -> Result<()> {
    let auth_header = read_auth_header_from_stdin()?;

    let upstream_url = Url::parse(&args.upstream_url).context("parsing --upstream-url")?;
    let host = match (upstream_url.host_str(), upstream_url.port()) {
        (Some(host), Some(port)) => format!("{host}:{port}"),
        (Some(host), None) => host.to_string(),
        _ => return Err(anyhow!("upstream URL must include a host")),
    };
    let host_header =
        HeaderValue::from_str(&host).context("constructing Host header from upstream URL")?;

    let forward_config = Arc::new(ForwardConfig {
        upstream_url,
        host_header,
    });
    let dump_dir = args
        .dump_dir
        .map(ExchangeDumper::new)
        .transpose()
        .context("creating --dump-dir")?
        .map(Arc::new);

    let (listener, bound_addr) = bind_listener(args.port)?;
    if let Some(path) = args.server_info.as_ref() {
        write_server_info(path, bound_addr.port())?;
    }
    let server = Server::from_listener(listener, None)
        .map_err(|err| anyhow!("creating HTTP server: {err}"))?;
    let client = Arc::new(
        Client::builder()
            // Disable reqwest's 30s default so long-lived response streams keep flowing.
            .timeout(None::<Duration>)
            .build()
            .context("building reqwest client")?,
    );

    eprintln!("responses-api-proxy listening on {bound_addr}");

    let http_shutdown = args.http_shutdown;
    for request in server.incoming_requests() {
        let client = client.clone();
        let forward_config = forward_config.clone();
        let dump_dir = dump_dir.clone();
        std::thread::spawn(move || {
            if http_shutdown && request.method() == &Method::Get && request.url() == "/shutdown" {
                let _ = request.respond(Response::new_empty(StatusCode(200)));
                std::process::exit(0);
            }

            if let Err(e) = forward_request(
                &client,
                auth_header,
                &forward_config,
                dump_dir.as_deref(),
                request,
            ) {
                eprintln!("forwarding error: {e}");
            }
        });
    }

    Err(anyhow!("server stopped unexpectedly"))
}

/// Test-friendly entry point that accepts an auth header as a parameter.
/// This is used in integration tests to avoid stdin redirection.
pub fn run_main_with_auth(args: Args, auth_header: &'static str) -> Result<()> {
    let upstream_url = Url::parse(&args.upstream_url).context("parsing --upstream-url")?;
    let host = match (upstream_url.host_str(), upstream_url.port()) {
        (Some(host), Some(port)) => format!("{host}:{port}"),
        (Some(host), None) => host.to_string(),
        _ => return Err(anyhow!("upstream URL must include a host")),
    };
    let host_header =
        HeaderValue::from_str(&host).context("constructing Host header from upstream URL")?;

    let forward_config = Arc::new(ForwardConfig {
        upstream_url,
        host_header,
    });
    let dump_dir = args
        .dump_dir
        .map(ExchangeDumper::new)
        .transpose()
        .context("creating --dump-dir")?
        .map(Arc::new);

    let (listener, bound_addr) = bind_listener(args.port)?;
    if let Some(path) = args.server_info.as_ref() {
        write_server_info(path, bound_addr.port())?;
    }
    let server = Server::from_listener(listener, None)
        .map_err(|err| anyhow!("creating HTTP server: {err}"))?;
    let client = Arc::new(
        Client::builder()
            .timeout(None::<Duration>)
            .build()
            .context("building reqwest client")?,
    );

    eprintln!("responses-api-proxy listening on {bound_addr}");

    let http_shutdown = args.http_shutdown;
    for request in server.incoming_requests() {
        let client = client.clone();
        let forward_config = forward_config.clone();
        let dump_dir = dump_dir.clone();
        std::thread::spawn(move || {
            if http_shutdown && request.method() == &Method::Get && request.url() == "/shutdown" {
                let _ = request.respond(Response::new_empty(StatusCode(200)));
                std::process::exit(0);
            }

            if let Err(e) = forward_request(
                &client,
                auth_header,
                &forward_config,
                dump_dir.as_deref(),
                request,
            ) {
                eprintln!("forwarding error: {e}");
            }
        });
    }

    Err(anyhow!("server stopped unexpectedly"))
}

/// Binds a TCP listener on the given port. If `port` is `None`, an ephemeral
/// port is used. Public for integration tests.
pub fn bind_listener(port: Option<u16>) -> Result<(TcpListener, SocketAddr)> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port.unwrap_or(0)));
    let listener = TcpListener::bind(addr).with_context(|| format!("failed to bind {addr}"))?;
    let bound = listener.local_addr().context("failed to read local_addr")?;
    Ok((listener, bound))
}

fn write_server_info(path: &Path, port: u16) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let info = ServerInfo {
        port,
        pid: std::process::id(),
    };
    let mut data = serde_json::to_string(&info)?;
    data.push('\n');
    let mut f = File::create(path)?;
    f.write_all(data.as_bytes())?;
    Ok(())
}

fn forward_request(
    client: &Client,
    auth_header: &'static str,
    config: &ForwardConfig,
    dump_dir: Option<&ExchangeDumper>,
    mut req: Request,
) -> Result<()> {
    // Only allow POST /v1/responses or POST /v1/chat/completions
    let method = req.method().clone();
    let url_path = req.url().to_string();
    let allow = method == Method::Post && (url_path == "/v1/responses" || url_path == "/v1/chat/completions");

    if !allow {
        let error_json = serde_json::json!({"error": "forbidden: route not supported"});
        let body = error_json.to_string();
        let body_len = body.len();
        let resp = Response::new(
            StatusCode(403),
            vec![Header::from_bytes(b"content-type", b"application/json").unwrap()],
            Box::new(Cursor::new(body.into_bytes())),
            Some(body_len),
            None,
        );
        let _ = req.respond(resp);
        return Ok(());
    }

    // Read request body
    let mut body = Vec::new();
    let reader = req.as_reader();
    reader.read_to_end(&mut body)?;

    let exchange_dump = dump_dir.and_then(|dump_dir| {
        dump_dir
            .dump_request(&method, &url_path, req.headers(), &body)
            .map_err(|err| {
                eprintln!("responses-api-proxy failed to dump request: {err}");
                err
            })
            .ok()
    });

    // Route to appropriate handler
    if url_path == "/v1/chat/completions" {
        return handle_chat_completions_internal(
            client,
            auth_header,
            config,
            req,
            &body,
            exchange_dump,
        );
    }

    // Existing /v1/responses route
    handle_responses_forward(client, auth_header, config, req, body, exchange_dump)
}

/// Handle POST /v1/responses (existing pass-through behavior)
fn handle_responses_forward(
    client: &Client,
    auth_header: &'static str,
    config: &ForwardConfig,
    req: Request,
    body: Vec<u8>,
    exchange_dump: Option<ExchangeDump>,
) -> Result<()> {
    // Build headers for upstream, forwarding everything from the incoming
    // request except Authorization (we replace it below).
    let mut headers = HeaderMap::new();
    for header in req.headers() {
        let name_ascii = header.field.as_str();
        let lower = name_ascii.to_ascii_lowercase();
        if lower.as_str() == "authorization" || lower.as_str() == "host" {
            continue;
        }

        let header_name = match HeaderName::from_bytes(lower.as_bytes()) {
            Ok(name) => name,
            Err(_) => continue,
        };
        if let Ok(value) = HeaderValue::from_bytes(header.value.as_bytes()) {
            headers.append(header_name, value);
        }
    }

    // As part of our effort to to keep `auth_header` secret, we use a
    // combination of `from_static()` and `set_sensitive(true)`.
    let mut auth_header_value = HeaderValue::from_static(auth_header);
    auth_header_value.set_sensitive(true);
    headers.insert(AUTHORIZATION, auth_header_value);

    headers.insert(HOST, config.host_header.clone());

    let upstream_resp = client
        .post(config.upstream_url.clone())
        .headers(headers)
        .body(body)
        .send()
        .context("forwarding request to upstream")?;

    // We have to create an adapter between a `reqwest::blocking::Response`
    // and a `tiny_http::Response`. Fortunately, `reqwest::blocking::Response`
    // implements `Read`, so we can use it directly as the body of the
    // `tiny_http::Response`.
    let status = upstream_resp.status();
    let mut response_headers = Vec::new();
    for (name, value) in upstream_resp.headers().iter() {
        // Skip headers that tiny_http manages itself.
        if matches!(
            name.as_str(),
            "content-length" | "transfer-encoding" | "connection" | "trailer" | "upgrade"
        ) {
            continue;
        }

        if let Ok(header) = Header::from_bytes(name.as_str().as_bytes(), value.as_bytes()) {
            response_headers.push(header);
        }
    }

    let content_length = upstream_resp.content_length().and_then(|len| {
        if len <= usize::MAX as u64 {
            Some(len as usize)
        } else {
            None
        }
    });

    let response_body: Box<dyn Read + Send> = if let Some(exchange_dump) = exchange_dump {
        let headers = upstream_resp.headers().clone();
        Box::new(exchange_dump.tee_response_body(status.as_u16(), &headers, upstream_resp))
    } else {
        Box::new(upstream_resp)
    };

    let response = Response::new(
        StatusCode(status.as_u16()),
        response_headers,
        response_body,
        content_length,
        None,
    );

    let _ = req.respond(response);
    Ok(())
}

/// Handle POST /v1/chat/completions (Chat Completions API)
fn handle_chat_completions_internal(
    client: &Client,
    auth_header: &'static str,
    config: &ForwardConfig,
    req: Request,
    body: &[u8],
    _exchange_dump: Option<ExchangeDump>,
) -> Result<()> {
    // Parse the ChatCompletionRequest
    let body_str = std::str::from_utf8(body).map_err(|e| {
        anyhow!("invalid UTF-8 in request body: {e}")
    })?;

    let chat_request: ChatCompletionRequest = match serde_json::from_str(body_str) {
        Ok(req) => req,
        Err(e) => {
            let error_json = serde_json::json!({"error": format!("invalid JSON: {e}")});
            let body = error_json.to_string();
            let body_len = body.len();
            let resp = Response::new(
                StatusCode(400),
                vec![Header::from_bytes(b"content-type", b"application/json").unwrap()],
                Box::new(Cursor::new(body.into_bytes())),
                Some(body_len),
                None,
            );
            let _ = req.respond(resp);
            return Ok(());
        }
    };

    // Convert Chat Completions request to Responses API request
    let responses_request = match convert_chat_to_responses(&chat_request) {
        Ok(req) => req,
        Err(e) => {
            let error_json = serde_json::json!({"error": format!("conversion failed: {e}")});
            let body = error_json.to_string();
            let body_len = body.len();
            let resp = Response::new(
                StatusCode(500),
                vec![Header::from_bytes(b"content-type", b"application/json").unwrap()],
                Box::new(Cursor::new(body.into_bytes())),
                Some(body_len),
                None,
            );
            let _ = req.respond(resp);
            return Ok(());
        }
    };

    // Serialize the Responses API request
    let request_body = match serde_json::to_vec(&responses_request) {
        Ok(body) => body,
        Err(e) => {
            let error_json = serde_json::json!({"error": format!("failed to encode request: {e}")});
            let body = error_json.to_string();
            let body_len = body.len();
            let resp = Response::new(
                StatusCode(500),
                vec![Header::from_bytes(b"content-type", b"application/json").unwrap()],
                Box::new(Cursor::new(body.into_bytes())),
                Some(body_len),
                None,
            );
            let _ = req.respond(resp);
            return Ok(());
        }
    };

    // Build headers for upstream
    let mut headers = HeaderMap::new();
    for header in req.headers() {
        let name_ascii = header.field.as_str();
        let lower = name_ascii.to_ascii_lowercase();
        if lower.as_str() == "authorization" || lower.as_str() == "host" {
            continue;
        }

        let header_name = match HeaderName::from_bytes(lower.as_bytes()) {
            Ok(name) => name,
            Err(_) => continue,
        };
        if let Ok(value) = HeaderValue::from_bytes(header.value.as_bytes()) {
            headers.append(header_name, value);
        }
    }

    let mut auth_header_value = HeaderValue::from_static(auth_header);
    auth_header_value.set_sensitive(true);
    headers.insert(AUTHORIZATION, auth_header_value);
    headers.insert(HOST, config.host_header.clone());

    // Forward to upstream Responses API
    let upstream_resp = client
        .post(config.upstream_url.clone())
        .headers(headers)
        .body(request_body)
        .send()
        .context("forwarding request to upstream")?;

    let status = upstream_resp.status();

    // Check for error status - pass through as-is
    if status.is_client_error() || status.is_server_error() {
        let mut response_headers = Vec::new();
        for (name, value) in upstream_resp.headers().iter() {
            if matches!(
                name.as_str(),
                "content-length" | "transfer-encoding" | "connection" | "trailer" | "upgrade"
            ) {
                continue;
            }
            if let Ok(header) = Header::from_bytes(name.as_str().as_bytes(), value.as_bytes()) {
                response_headers.push(header);
            }
        }

        let content_length = upstream_resp.content_length().and_then(|len| {
            if len <= usize::MAX as u64 { Some(len as usize) } else { None }
        });

        let response_body: Box<dyn Read + Send> = Box::new(upstream_resp);

        let response = Response::new(
            StatusCode(status.as_u16()),
            response_headers,
            response_body,
            content_length,
            None,
        );
        let _ = req.respond(response);
        return Ok(());
    }

    // Handle streaming vs non-streaming
    if chat_request.stream.unwrap_or(false) {
        handle_chat_streaming(req, upstream_resp, chat_request.model)
    } else {
        handle_chat_non_streaming(upstream_resp, req, chat_request.model)
    }
}

/// Handle non-streaming chat completion response
fn handle_chat_non_streaming(
    upstream_resp: reqwest::blocking::Response,
    req: Request,
    model: String,
) -> Result<()> {
    // Read the full response body
    let body_bytes = upstream_resp.bytes().context("reading upstream response body")?;

    // Parse as Responses API response
    #[derive(Deserialize)]
    struct ResponsesResponse {
        id: String,
        #[allow(dead_code)]
        model: Option<String>,
        #[serde(default)]
        created_at: Option<u64>,
        #[serde(default)]
        output: Vec<codex_protocol::models::ResponseItem>,
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        usage: Option<codex_protocol::protocol::TokenUsage>,
    }

    let responses_resp: ResponsesResponse = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(_) => {
            // If parsing fails, return the raw response as-is
            let response_headers = vec![
                Header::from_bytes(b"content-type", b"application/json").unwrap()
            ];
            let body = Cursor::new(body_bytes.to_vec());
            let response = Response::new(
                StatusCode(200),
                response_headers,
                Box::new(body),
                None,
                None,
            );
            let _ = req.respond(response);
            return Ok(());
        }
    };

    // Convert Responses API response to Chat Completions response
    let chat_response = convert_responses_to_chat(
        &responses_resp.id,
        &model,
        responses_resp.created_at.unwrap_or(0),
        &responses_resp.output,
        responses_resp.status.as_deref().unwrap_or("completed"),
        responses_resp.usage.as_ref(),
    );

    // Serialize the Chat Completions response
    let response_body = serde_json::to_vec(&chat_response)
        .context("serializing chat completion response")?;

    // Build response headers
    let response_headers = vec![
        Header::from_bytes(b"content-type", b"application/json").unwrap()
    ];

    let response = Response::new(
        StatusCode(200),
        response_headers,
        Box::new(Cursor::new(response_body)),
        None,
        None,
    );

    let _ = req.respond(response);
    Ok(())
}

/// Handle streaming chat completion response
fn handle_chat_streaming(
    req: Request,
    upstream_resp: reqwest::blocking::Response,
    model: String,
) -> Result<()> {
    // Build response headers for SSE
    let response_headers = vec![
        Header::from_bytes(b"content-type", b"text/event-stream").unwrap(),
        Header::from_bytes(b"cache-control", b"no-cache").unwrap(),
    ];

    // Create a streaming body that converts SSE events
    let stream_body = ChatCompletionStreamBody::new(upstream_resp, model);

    let response = Response::new(
        StatusCode(200),
        response_headers,
        Box::new(stream_body),
        None,
        None,
    );

    let _ = req.respond(response);
    Ok(())
}

/// Streaming body reader that converts Responses API SSE to Chat Completions SSE
struct ChatCompletionStreamBody {
    reader: BufReader<reqwest::blocking::Response>,
    model: String,
    response_id: Option<String>,
    created: u64,
    first_delta_sent: bool,
    completed_sent: bool,
    buffer: String,
}

impl ChatCompletionStreamBody {
    fn new(upstream_resp: reqwest::blocking::Response, model: String) -> Self {
        Self {
            reader: BufReader::new(upstream_resp),
            model,
            response_id: None,
            created: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            first_delta_sent: false,
            completed_sent: false,
            buffer: String::new(),
        }
    }

    fn parse_sse_event(&mut self) -> Result<Option<SseEvent>> {
        let mut data = String::new();
        let mut event_end = 0;

        for (i, line) in self.buffer.lines().enumerate() {
            if line.is_empty() {
                // Empty line marks end of event
                if !data.is_empty() {
                    let event = SseEvent { data };
                    // Find the end of this event (the position after the empty line)
                    event_end = if i > 0 {
                        // Approximate: remove up to and including this empty line
                        self.buffer
                            .lines()
                            .take(i + 1)
                            .map(|l| l.len() + 1) // +1 for \n
                            .sum()
                    } else {
                        0
                    };
                    self.buffer = self.buffer[event_end..].to_string();
                    return Ok(Some(event));
                }
                continue;
            }

            if let Some(rest) = line.strip_prefix("data: ") {
                data = rest.to_string();
            }
        }

        // No complete event found - keep buffer as-is for more data
        Ok(None)
    }

    fn convert_responses_event_to_chat(&mut self, event: &SseEvent) -> Result<Vec<String>> {
        use serde_json::json;

        #[derive(Deserialize)]
        struct ResponsesStreamEvent {
            #[serde(rename = "type")]
            kind: String,
            #[serde(default)]
            response: Option<serde_json::Value>,
            #[serde(default)]
            delta: Option<String>,
            #[serde(default)]
            item_id: Option<String>,
            #[serde(default)]
            call_id: Option<String>,
        }

        let responses_event: ResponsesStreamEvent = serde_json::from_str(&event.data)
            .map_err(|e| anyhow!("failed to parse SSE event: {e}"))?;

        let mut chat_events = Vec::new();

        match responses_event.kind.as_str() {
            "response.created" => {
                if let Some(resp) = &responses_event.response {
                    if let Some(id) = resp.get("id").and_then(|v| v.as_str()) {
                        self.response_id = Some(format!("chatcmpl-{}", id));
                    }
                }
                // Send initial role delta
                let chunk = json!({
                    "id": self.response_id.as_deref().unwrap_or("chatcmpl-unknown"),
                    "object": "chat.completion.chunk",
                    "created": self.created,
                    "model": self.model,
                    "choices": [{
                        "index": 0,
                        "delta": {"role": "assistant"},
                        "finish_reason": null
                    }]
                });
                chat_events.push(chunk.to_string());
                self.first_delta_sent = true;
            }
            "response.output_text.delta" => {
                // Ensure we've sent the role delta first
                if !self.first_delta_sent {
                    let chunk = json!({
                        "id": self.response_id.as_deref().unwrap_or("chatcmpl-unknown"),
                        "object": "chat.completion.chunk",
                        "created": self.created,
                        "model": self.model,
                        "choices": [{
                            "index": 0,
                            "delta": {"role": "assistant"},
                            "finish_reason": null
                        }]
                    });
                    chat_events.push(chunk.to_string());
                    self.first_delta_sent = true;
                }

                if let Some(delta) = responses_event.delta {
                    let chunk = json!({
                        "id": self.response_id.as_deref().unwrap_or("chatcmpl-unknown"),
                        "object": "chat.completion.chunk",
                        "created": self.created,
                        "model": self.model,
                        "choices": [{
                            "index": 0,
                            "delta": {"content": delta},
                            "finish_reason": null
                        }]
                    });
                    chat_events.push(chunk.to_string());
                }
            }
            "response.custom_tool_call_input.delta" => {
                let call_id = responses_event.call_id.unwrap_or_else(|| {
                    responses_event.item_id.unwrap_or_else(|| "unknown".to_string())
                });
                let delta = responses_event.delta.unwrap_or_default();

                let chunk = json!({
                    "id": self.response_id.as_deref().unwrap_or("chatcmpl-unknown"),
                    "object": "chat.completion.chunk",
                    "created": self.created,
                    "model": self.model,
                    "choices": [{
                        "index": 0,
                        "delta": {
                            "tool_calls": [{
                                "index": 0,
                                "id": call_id,
                                "type": "function",
                                "function": {"arguments": delta}
                            }]
                        },
                        "finish_reason": null
                    }]
                });
                chat_events.push(chunk.to_string());
            }
            "response.completed" => {
                if let Some(resp) = &responses_event.response {
                    if let Some(id) = resp.get("id").and_then(|v| v.as_str()) {
                        self.response_id = Some(format!("chatcmpl-{}", id));
                    }

                    let end_turn = resp.get("end_turn").and_then(|v| v.as_bool());
                    let finish_reason = match end_turn {
                        Some(false) => "length",
                        Some(true) | None => "stop",
                    };

                    // Send chunk with finish_reason
                    let chunk = json!({
                        "id": self.response_id.as_deref().unwrap_or("chatcmpl-unknown"),
                        "object": "chat.completion.chunk",
                        "created": self.created,
                        "model": self.model,
                        "choices": [{
                            "index": 0,
                            "delta": {},
                            "finish_reason": finish_reason
                        }]
                    });
                    chat_events.push(chunk.to_string());

                    // Send usage if present
                    if let Some(usage_val) = resp.get("usage") {
                        if let Ok(usage) = serde_json::from_value::<codex_protocol::protocol::TokenUsage>(usage_val.clone()) {
                            let chunk = json!({
                                "id": self.response_id.as_deref().unwrap_or("chatcmpl-unknown"),
                                "object": "chat.completion.chunk",
                                "created": self.created,
                                "model": self.model,
                                "choices": [{
                                    "index": 0,
                                    "delta": {},
                                    "finish_reason": null
                                }],
                                "usage": {
                                    "prompt_tokens": usage.input_tokens,
                                    "completion_tokens": usage.output_tokens,
                                    "total_tokens": usage.total_tokens
                                }
                            });
                            chat_events.push(chunk.to_string());
                        }
                    }

                    // Send [DONE]
                    chat_events.push("[DONE]".to_string());
                    self.completed_sent = true;
                }
            }
            _ => {
                // Ignore other event types
            }
        }

        Ok(chat_events)
    }
}

struct SseEvent {
    data: String,
}

impl Read for ChatCompletionStreamBody {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.completed_sent {
            return Ok(0); // EOF
        }

        let mut output = String::new();

        loop {
            // Read more data from upstream
            let mut line = String::new();
            let bytes_read = self.reader.read_line(&mut line)?;

            if bytes_read == 0 {
                // EOF from upstream - send [DONE] if we haven't already
                if !self.completed_sent {
                    output.push_str("data: [DONE]\n\n");
                    self.completed_sent = true;
                }
                break;
            }

            self.buffer.push_str(&line);

            // Try to parse a complete SSE event
            if let Some(event) = self.parse_sse_event().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("SSE parse error: {e}"))
            })? {
                // Convert the event
                match self.convert_responses_event_to_chat(&event) {
                    Ok(chat_events) => {
                        for chat_event in chat_events {
                            if chat_event == "[DONE]" {
                                output.push_str("data: [DONE]\n\n");
                                self.completed_sent = true;
                                break;
                            } else {
                                output.push_str(&format!("data: {}\n\n", chat_event));
                            }
                        }
                        if self.completed_sent {
                            break;
                        }
                        if !output.is_empty() && output.len() > 4096 {
                            // Don't buffer too much
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error converting SSE event: {e}");
                        // Continue processing other events
                    }
                }
            }
        }

        if output.is_empty() {
            Ok(0) // EOF
        } else {
            let bytes = output.as_bytes();
            let len = bytes.len().min(buf.len());
            buf[..len].copy_from_slice(&bytes[..len]);
            Ok(len)
        }
    }
}
