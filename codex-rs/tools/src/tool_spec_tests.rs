use super::ResponsesApiNamespace;
use super::ResponsesApiWebSearchFilters;
use super::ResponsesApiWebSearchUserLocation;
use super::ToolSpec;
use crate::AdditionalProperties;
use crate::FreeformTool;
use crate::FreeformToolFormat;
use crate::JsonSchema;
use crate::ResponsesApiNamespaceTool;
use crate::ResponsesApiTool;
use crate::create_tools_json_for_chat_completions_api;
use crate::create_tools_json_for_responses_api;
use codex_protocol::config_types::WebSearchContextSize;
use codex_protocol::config_types::WebSearchFilters as ConfigWebSearchFilters;
use codex_protocol::config_types::WebSearchUserLocation as ConfigWebSearchUserLocation;
use codex_protocol::config_types::WebSearchUserLocationType;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn tool_spec_name_covers_all_variants() {
    assert_eq!(
        ToolSpec::Function(ResponsesApiTool {
            name: "lookup_order".to_string(),
            description: "Look up an order".to_string(),
            strict: false,
            defer_loading: None,
            parameters: JsonSchema::object(
                BTreeMap::new(),
                /*required*/ None,
                /*additional_properties*/ None
            ),
            output_schema: None,
        })
        .name(),
        "lookup_order"
    );
    assert_eq!(
        ToolSpec::Namespace(ResponsesApiNamespace {
            name: "mcp__demo__".to_string(),
            description: "Demo tools".to_string(),
            tools: Vec::new(),
        })
        .name(),
        "mcp__demo__"
    );
    assert_eq!(
        ToolSpec::ToolSearch {
            execution: "sync".to_string(),
            description: "Search for tools".to_string(),
            parameters: JsonSchema::object(
                BTreeMap::new(),
                /*required*/ None,
                /*additional_properties*/ None
            ),
        }
        .name(),
        "tool_search"
    );
    assert_eq!(
        ToolSpec::ImageGeneration {
            output_format: "png".to_string(),
        }
        .name(),
        "image_generation"
    );
    assert_eq!(
        ToolSpec::WebSearch {
            external_web_access: Some(true),
            index_gated_web_access: None,
            filters: None,
            user_location: None,
            search_context_size: None,
            search_content_types: None,
        }
        .name(),
        "web_search"
    );
    assert_eq!(
        ToolSpec::Freeform(FreeformTool {
            name: "exec".to_string(),
            description: "Run a command".to_string(),
            format: FreeformToolFormat {
                r#type: "grammar".to_string(),
                syntax: "lark".to_string(),
                definition: "start: \"exec\"".to_string(),
            },
        })
        .name(),
        "exec"
    );
}

#[test]
fn web_search_config_converts_to_responses_api_types() {
    assert_eq!(
        ResponsesApiWebSearchFilters::from(ConfigWebSearchFilters {
            allowed_domains: Some(vec!["example.com".to_string()]),
        }),
        ResponsesApiWebSearchFilters {
            allowed_domains: Some(vec!["example.com".to_string()]),
        }
    );
    assert_eq!(
        ResponsesApiWebSearchUserLocation::from(ConfigWebSearchUserLocation {
            r#type: WebSearchUserLocationType::Approximate,
            country: Some("US".to_string()),
            region: Some("California".to_string()),
            city: Some("San Francisco".to_string()),
            timezone: Some("America/Los_Angeles".to_string()),
        }),
        ResponsesApiWebSearchUserLocation {
            r#type: WebSearchUserLocationType::Approximate,
            country: Some("US".to_string()),
            region: Some("California".to_string()),
            city: Some("San Francisco".to_string()),
            timezone: Some("America/Los_Angeles".to_string()),
        }
    );
}

#[test]
fn create_tools_json_for_responses_api_includes_top_level_name() {
    assert_eq!(
        create_tools_json_for_responses_api(&[ToolSpec::Function(ResponsesApiTool {
            name: "demo".to_string(),
            description: "A demo tool".to_string(),
            strict: false,
            defer_loading: None,
            parameters: JsonSchema::object(
                BTreeMap::from([("foo".to_string(), JsonSchema::string(/*description*/ None),)]),
                /*required*/ None,
                /*additional_properties*/ None
            ),
            output_schema: None,
        })])
        .expect("serialize tools"),
        vec![json!({
            "type": "function",
            "name": "demo",
            "description": "A demo tool",
            "parameters": {
                "type": "object",
                "properties": {
                    "foo": { "type": "string" }
                },
            },
        })]
    );
}

#[test]
fn namespace_tool_spec_serializes_expected_wire_shape() {
    assert_eq!(
        serde_json::to_value(ToolSpec::Namespace(ResponsesApiNamespace {
            name: "mcp__demo__".to_string(),
            description: "Demo tools".to_string(),
            tools: vec![ResponsesApiNamespaceTool::Function(ResponsesApiTool {
                name: "lookup_order".to_string(),
                description: "Look up an order".to_string(),
                strict: false,
                defer_loading: None,
                parameters: JsonSchema::object(
                    BTreeMap::from([(
                        "order_id".to_string(),
                        JsonSchema::string(/*description*/ None),
                    )]),
                    /*required*/ None,
                    /*additional_properties*/ None,
                ),
                output_schema: None,
            })],
        }))
        .expect("serialize namespace tool"),
        json!({
            "type": "namespace",
            "name": "mcp__demo__",
            "description": "Demo tools",
            "tools": [
                {
                    "type": "function",
                    "name": "lookup_order",
                    "description": "Look up an order",
                    "strict": false,
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "order_id": { "type": "string" },
                        },
                    },
                },
            ],
        })
    );
}

#[test]
fn web_search_tool_spec_serializes_expected_wire_shape() {
    assert_eq!(
        serde_json::to_value(ToolSpec::WebSearch {
            external_web_access: Some(true),
            index_gated_web_access: None,
            filters: Some(ResponsesApiWebSearchFilters {
                allowed_domains: Some(vec!["example.com".to_string()]),
            }),
            user_location: Some(ResponsesApiWebSearchUserLocation {
                r#type: WebSearchUserLocationType::Approximate,
                country: Some("US".to_string()),
                region: Some("California".to_string()),
                city: Some("San Francisco".to_string()),
                timezone: Some("America/Los_Angeles".to_string()),
            }),
            search_context_size: Some(WebSearchContextSize::High),
            search_content_types: Some(vec!["text".to_string(), "image".to_string()]),
        })
        .expect("serialize web_search"),
        json!({
            "type": "web_search",
            "external_web_access": true,
            "filters": {
                "allowed_domains": ["example.com"],
            },
            "user_location": {
                "type": "approximate",
                "country": "US",
                "region": "California",
                "city": "San Francisco",
                "timezone": "America/Los_Angeles",
            },
            "search_context_size": "high",
            "search_content_types": ["text", "image"],
        })
    );
}

#[test]
fn tool_search_tool_spec_serializes_expected_wire_shape() {
    assert_eq!(
        serde_json::to_value(ToolSpec::ToolSearch {
            execution: "sync".to_string(),
            description: "Search app tools".to_string(),
            parameters: JsonSchema::object(
                BTreeMap::from([(
                    "query".to_string(),
                    JsonSchema::string(Some("Tool search query".to_string()),),
                )]),
                Some(vec!["query".to_string()]),
                Some(AdditionalProperties::Boolean(false))
            ),
        })
        .expect("serialize tool_search"),
        json!({
            "type": "tool_search",
            "execution": "sync",
            "description": "Search app tools",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Tool search query",
                    }
                },
                "required": ["query"],
                "additionalProperties": false,
            },
        })
    );
}


#[test]
fn create_tools_json_for_chat_completions_api_wraps_name_inside_function() {
    // Regression test for: third-party providers (Qwen, DashScope, Zhipu) reject the
    // Chat Completions request with
    //   {"error": {"message": "'name' is a required property - 'tools.0.function'"}}
    // when `tools[0].function.name` is missing. The tool spec is therefore expected
    // to look like:
    //   { "type": "function", "function": { "name": "...", "description": "...",
    //                                       "parameters": { ... } } }
    // (Responses-API-shaped tools with the name at the top level must NOT leak through.)
    let tools = vec![ToolSpec::Function(ResponsesApiTool {
        name: "lookup_order".to_string(),
        description: "Look up an order".to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            BTreeMap::from([(
                "order_id".to_string(),
                JsonSchema::string(/*description*/ None),
            )]),
            /*required*/ None,
            /*additional_properties*/ None,
        ),
        output_schema: None,
    })];

    let chat_tools = create_tools_json_for_chat_completions_api(&tools)
        .expect("serialize chat tools");

    assert_eq!(
        chat_tools,
        vec![json!({
            "type": "function",
            "function": {
                "name": "lookup_order",
                "description": "Look up an order",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "order_id": { "type": "string" },
                    },
                },
            },
        })]
    );
}

#[test]
fn create_tools_json_for_chat_completions_api_drops_non_function_tools() {
    // Web search, namespace, image_generation, and tool_search are not supported on
    // the Chat Completions API and must be filtered out instead of leaking through
    // as a malformed `function` entry.
    let tools = vec![
        ToolSpec::WebSearch {
            external_web_access: None,
            filters: None,
            user_location: None,
            search_context_size: None,
            search_content_types: None,
        },
        ToolSpec::Function(ResponsesApiTool {
            name: "echo".to_string(),
            description: "Echo a string".to_string(),
            strict: false,
            defer_loading: None,
            parameters: JsonSchema::object(
                BTreeMap::new(),
                /*required*/ None,
                /*additional_properties*/ None,
            ),
            output_schema: None,
        }),
    ];

    let chat_tools = create_tools_json_for_chat_completions_api(&tools)
        .expect("serialize chat tools");

    assert_eq!(
        chat_tools,
        vec![json!({
            "type": "function",
            "function": {
                "name": "echo",
                "description": "Echo a string",
                "parameters": {
                    "type": "object",
                    "properties": {},
                },
            },
        })]
    );
}

#[test]
fn end_to_end_chat_completions_body_has_function_name_for_qwen_dashscope() {
    // End-to-end shape: this is the exact JSON the codex-api `ChatRequestBuilder`
    // sends on the wire for a single function tool. Third-party providers such
    // as Qwen / DashScope / Zhipu reject the request with
    //   {"error": {"message": "'name' is a required property - 'tools.0.function'"}}
    // when `tools[0].function.name` is missing. The shape below is the one that
    // satisfies that constraint: `name` lives inside the inner `function` object,
    // not at the top level of `tools[0]`.
    let tools = vec![ToolSpec::Function(ResponsesApiTool {
        name: "lookup_order".to_string(),
        description: "Look up an order".to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            BTreeMap::from([(
                "order_id".to_string(),
                JsonSchema::string(/*description*/ None),
            )]),
            /*required*/ None,
            /*additional_properties*/ None,
        ),
        output_schema: None,
    })];

    let chat_tools = create_tools_json_for_chat_completions_api(&tools)
        .expect("serialize chat tools");

    // Simulate the final body the way codex-api would post it.
    let body = json!({
        "model": "qwen-plus",
        "messages": [],
        "stream": true,
        "tools": chat_tools,
    });

    // Walk the path Qwen reported as missing `name`.
    let tools0 = body
        .get("tools")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .expect("tools[0] exists");
    let name = tools0
        .get("function")
        .and_then(|f| f.get("name"))
        .and_then(|v| v.as_str());
    assert_eq!(
        name,
        Some("lookup_order"),
        "tools[0].function.name must be set so Qwen accepts the request"
    );
    // Sanity: the Responses-API-shaped top-level tool must NOT leak into
    // `function` (which would push `name` one level deeper and trigger the
    // 'name' is a required property - tools.0.function error).
    assert!(tools0.get("function").and_then(|f| f.get("type")).is_none());
}
