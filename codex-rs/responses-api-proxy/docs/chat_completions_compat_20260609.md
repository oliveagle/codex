# Chat Completions API Compatibility

## Overview

The `responses-api-proxy` now supports the OpenAI Chat Completions API format alongside the existing Codex Responses API. This compatibility layer allows clients using the standard `POST /v1/chat/completions` endpoint to work with the Codex Responses API through an internal bidirectional conversion.

## Supported Endpoint

```
POST /v1/chat/completions
```

## Architecture

The proxy implements an internal conversion layer that translates between the two API formats:

```
Client Request (Chat Completions)
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│              responses-api-proxy                         │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  Chat Completions Request  ──►  Responses API Request   │
│         │                           │                   │
│         │                           ▼                   │
│         │                   Upstream (OpenAI/Codex)      │
│         │                           │                   │
│         │                           ▼                   │
│         │                   Responses API Response       │
│         │                           │                   │
│         ▼                           │                   │
│  Chat Completions Response  ◄──  Responses API Response │
│                                                          │
└─────────────────────────────────────────────────────────┘
         │
         ▼
Client Response (Chat Completions)
```

## Supported Request Fields

The following Chat Completions API request fields are supported:

| Field | Type | Support | Notes |
|-------|------|---------|-------|
| `model` | string | ✅ Full | Passed through to Responses API |
| `messages` | array | ✅ Full | Converted to `input` and `instructions` |
| `stream` | boolean | ✅ Full | SSE streaming supported |
| `tools` | array | ✅ Full | Function calling supported |
| `tool_choice` | string/object | ✅ Full | `auto`, `none`, `required`, specific function |
| `temperature` | number | ⚠️ Ignored | Not applicable to Responses API |
| `top_p` | number | ⚠️ Ignored | Not applicable to Responses API |
| `max_tokens` | number | ⚠️ Ignored | Not applicable to Responses API |
| `max_completion_tokens` | number | ⚠️ Ignored | Not applicable to Responses API |
| `stop` | array/string | ⚠️ Ignored | Not applicable to Responses API |

### Message Roles

The following message roles are supported:

| Role | Conversion | Notes |
|------|-------------|-------|
| `system` | → `instructions` | Extracted to `instructions` field (concatenated with "\n\n") |
| `user` | → `ResponseItem::Message` | Added to `input` array with `role: "user"` |
| `assistant` | → `ResponseItem::Message` or `FunctionCall` | Added to `input` array. If `tool_calls` present, split into separate `FunctionCall` items |
| `tool` | → `ResponseItem::FunctionCallOutput` | Requires `tool_call_id` field |

### Tool Calls

Function calling is fully supported with bidirectional conversion:

**Request Direction (Chat → Responses)**:
- `tools` array → Converted to Responses API `tools` format
- `tool_choice` → Converted to string representation
- Assistant messages with `tool_calls` → Split into separate `FunctionCall` items
- Tool messages (role=tool) → Converted to `FunctionCallOutput`

**Response Direction (Responses → Chat)**:
- `ResponseItem::FunctionCall` → `tool_calls` array in response
- `finish_reason` → Set to `tool_calls` when function calls present
- `id` → Tool call `call_id` is preserved

## Streaming Support

Both streaming and non-streaming modes are supported:

### Non-Streaming (`stream: false`)

Returns a complete `ChatCompletionResponse` JSON object:

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "Hello, world!"
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 5,
    "total_tokens": 15
  }
}
```

### Streaming (`stream: true`)

Returns Server-Sent Events (SSE) with `data:` lines:

```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

## Response Format

### Finish Reason Mapping

| Responses API Status | Chat Completions `finish_reason` |
|---------------------|----------------------------------|
| `completed` (no tools) | `stop` |
| `completed` (with tools) | `tool_calls` |
| `incomplete` | `length` |
| `failed` | `stop` |
| Other | `stop` |

### Usage Mapping

| Responses API Field | Chat Completions Field |
|-------------------|------------------------|
| `input_tokens` | `prompt_tokens` |
| `output_tokens` | `completion_tokens` |
| `total_tokens` | `total_tokens` |

## Examples

### cURL - Non-Streaming

```bash
curl -X POST http://127.0.0.1:60001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "What is the capital of France?"}
    ]
  }'
```

### cURL - Streaming

```bash
curl -X POST http://127.0.0.1:60001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "user", "content": "Tell me a joke"}
    ],
    "stream": true
  }'
```

### cURL - Function Calling

```bash
curl -X POST http://127.0.0.1:60001/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "user", "content": "What'\''s the weather in San Francisco?"}
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "get_weather",
          "description": "Get current weather for a location",
          "parameters": {
            "type": "object",
            "properties": {
              "location": {"type": "string"}
            },
            "required": ["location"]
          }
        }
      }
    ]
  }'
```

### Python SDK

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://127.0.0.1:60001/v1",
    api_key="your-api-key"
)

response = client.chat.completions.create(
    model="gpt-4",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Explain quantum computing"}
    ]
)

print(response.choices[0].message.content)
```

### Python SDK - Function Calling

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://127.0.0.1:60001/v1",
    api_key="your-api-key"
)

response = client.chat.completions.create(
    model="gpt-4",
    messages=[
        {"role": "user", "content": "What'\''s the weather in Tokyo?"}
    ],
    tools=[
        {
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    },
                    "required": ["location"]
                }
            }
        }
    ]
)

tool_call = response.choices[0].message.tool_calls[0]
print(tool_call.function.name)
print(tool_call.function.arguments)
```

### Node.js SDK

```javascript
import OpenAI from 'openai';

const client = new OpenAI({
  baseURL: 'http://127.0.0.1:60001/v1',
  apiKey: 'your-api-key'
});

const response = await client.chat.completions.create({
  model: 'gpt-4',
  messages: [
    { role: 'system', content: 'You are a helpful assistant.' },
    { role: 'user', content: 'Write a haiku about coding' }
  ]
});

console.log(response.choices[0].message.content);
```

### Node.js SDK - Streaming

```javascript
import OpenAI from 'openai';

const client = new OpenAI({
  baseURL: 'http://127.0.0.1:60001/v1',
  apiKey: 'your-api-key'
});

const stream = await client.chat.completions.create({
  model: 'gpt-4',
  messages: [
    { role: 'user', content: 'Count from 1 to 10' }
  ],
  stream: true
});

for await (const chunk of stream) {
  const content = chunk.choices[0]?.delta?.content || '';
  process.stdout.write(content);
}
```

## Error Codes

| HTTP Status | Error Type | Description |
|-------------|-----------|-------------|
| `400` | Bad Request | Invalid JSON in request body |
| `400` | Bad Request | Conversion failed (e.g., missing tool_call_id) |
| `403` | Forbidden | Route not supported (only `/v1/responses` and `/v1/chat/completions` allowed) |
| `4xx` | Upstream Error | Client errors from upstream API (passed through) |
| `5xx` | Upstream Error | Server errors from upstream API (passed through) |

## Implementation Notes

### Conversion Modules

The conversion logic is implemented in separate modules for clarity:

- **`codex-api/src/conversion/chat_to_responses.rs`**: Chat Completions → Responses API
- **`codex-api/src/conversion/responses_to_chat.rs`**: Responses API → Chat Completions (non-streaming)
- **`codex-api/src/conversion/responses_to_chat_sse.rs`**: Responses API → Chat Completions (streaming)

### Internal Request Flow

1. Receive Chat Completions request at `POST /v1/chat/completions`
2. Parse `ChatCompletionRequest` from JSON
3. Convert to `ResponsesApiRequest` using `convert_chat_to_responses()`
4. Serialize and forward to upstream Responses API
5. Receive Responses API response
6. If `stream: false`: Convert entire response using `convert_responses_to_chat()`
7. If `stream: true`: Stream and convert each SSE event using `ChatCompletionStreamBody`
8. Return Chat Completions response to client

### Limitations

1. **Parameter Support**: Parameters like `temperature`, `top_p`, `max_tokens`, `stop` are silently ignored as they don't have equivalents in the Responses API.

2. **Tool Types**: Only `type: "function"` is supported. Other tool types (e.g., `retrieval`, `code_interpreter`) are not implemented.

3. **Content Types**: Only text content is fully supported. Image and other content types are ignored in the response conversion.

4. **Single Choice**: The implementation only supports `choices[0]` (single choice responses). The `n` parameter for multiple choices is not supported.

5. **Logprobs**: Log probabilities are not supported.

6. **Response ID**: Response IDs are prefixed with `chatcmpl-` to match Chat Completions API conventions.

## Testing

The conversion modules include comprehensive unit tests covering:

- Simple text responses
- Multi-turn conversations
- System message extraction
- Tool calls and function calling
- Streaming responses
- Error cases (missing tool_call_id, etc.)

Run tests with:

```bash
cd codex-rs
cargo test --package codex-api --lib conversion
cargo test --package codex-responses-api-proxy
```

## Related Documentation

- [README.md](../README.md) - Main proxy documentation
- [OpenAI Chat Completions API](https://platform.openai.com/docs/api-reference/chat)
- [Codex Responses API](https://github.com/openai/codex) - Internal API documentation
