use crate::auth::SharedAuthProvider;
use crate::common::ResponseStream;
use crate::endpoint::session::EndpointSession;
use crate::error::ApiError;
use crate::provider::Provider;
use crate::requests::chat::ChatRequest;
use crate::requests::chat::ChatRequestBuilder;
use crate::sse::chat::spawn_chat_stream;
use crate::telemetry::SseTelemetry;
use codex_client::EncodedJsonBody;
use codex_client::HttpTransport;
use codex_client::RequestCompression;
use codex_client::RequestTelemetry;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::SessionSource;
use http::HeaderMap;
use http::Method;
use serde_json::Value;
use std::sync::Arc;

pub struct ChatClient<T: HttpTransport> {
    session: EndpointSession<T>,
    sse_telemetry: Option<Arc<dyn SseTelemetry>>,
}

impl<T: HttpTransport> ChatClient<T> {
    pub fn new(transport: T, provider: Provider, auth: SharedAuthProvider) -> Self {
        Self {
            session: EndpointSession::new(transport, provider, auth),
            sse_telemetry: None,
        }
    }

    pub fn with_telemetry(
        self,
        request: Option<Arc<dyn RequestTelemetry>>,
        sse: Option<Arc<dyn SseTelemetry>>,
    ) -> Self {
        Self {
            session: self.session.with_request_telemetry(request),
            sse_telemetry: sse,
        }
    }

    pub async fn stream_request(
        &self,
        request: ChatRequest,
    ) -> Result<ResponseStream, ApiError> {
        self.stream(request.body, request.headers).await
    }

    pub async fn stream_prompt(
        &self,
        model: &str,
        instructions: &str,
        input: &[ResponseItem],
        tools: &[Value],
        conversation_id: Option<String>,
        session_source: Option<SessionSource>,
    ) -> Result<ResponseStream, ApiError> {
        let provider = self.session.provider();
        let request = ChatRequestBuilder::new(model, instructions, input, tools)
            .conversation_id(conversation_id)
            .session_source(session_source)
            .build(provider)?;

        self.stream_request(request).await
    }

    pub async fn stream(
        &self,
        body: Value,
        extra_headers: HeaderMap,
    ) -> Result<ResponseStream, ApiError> {
        let encoded_body = EncodedJsonBody::encode(&body)
            .map_err(|e| ApiError::Stream(format!("failed to encode request: {e}")))?;

        let stream_response = self
            .session
            .stream_encoded_json_with(
                Method::POST,
                Self::path(),
                extra_headers,
                Some(encoded_body),
                |req| {
                    req.headers.insert(
                        http::header::ACCEPT,
                        http::HeaderValue::from_static("text/event-stream"),
                    );
                    req.compression = RequestCompression::None;
                },
            )
            .await?;

        Ok(spawn_chat_stream(
            stream_response,
            self.session.provider().stream_idle_timeout,
            self.sse_telemetry.clone(),
            None,
        ))
    }

    fn path() -> &'static str {
        "chat/completions"
    }
}
