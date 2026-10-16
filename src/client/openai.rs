use super::*;

use crate::utils::strip_think_tag;

use anyhow::{bail, Context, Result};
use reqwest::RequestBuilder;
use serde::Deserialize;
use serde_json::{json, Value};

const API_BASE: &str = "https://api.openai.com/v1";

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OpenAIConfig {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization_id: Option<String>,
    #[serde(default)]
    pub models: Vec<ModelData>,
    pub patch: Option<RequestPatch>,
    pub extra: Option<ExtraConfig>,
}

impl OpenAIClient {
    config_get_fn!(api_key, get_api_key);
    config_get_fn!(api_base, get_api_base);

    pub const PROMPTS: [PromptAction<'static>; 1] = [("api_key", "API Key", None)];
}

impl_client_trait!(
    OpenAIClient,
    (
        prepare_chat_completions,
        openai_chat_completions,
        openai_chat_completions_streaming
    ),
    (prepare_embeddings, openai_embeddings),
    (noop_prepare_rerank, noop_rerank),
);

fn prepare_chat_completions(
    self_: &OpenAIClient,
    data: ChatCompletionsData,
) -> Result<RequestData> {
    let api_key = self_.get_api_key()?;
    let api_base = self_
        .get_api_base()
        .unwrap_or_else(|_| API_BASE.to_string());

    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));

    let body = openai_build_chat_completions_body(data, &self_.model);

    let mut request_data = RequestData::new(url, body);

    request_data.bearer_auth(api_key);
    if let Some(organization_id) = &self_.config.organization_id {
        request_data.header("OpenAI-Organization", organization_id);
    }

    Ok(request_data)
}

fn prepare_embeddings(self_: &OpenAIClient, data: &EmbeddingsData) -> Result<RequestData> {
    let api_key = self_.get_api_key()?;
    let api_base = self_
        .get_api_base()
        .unwrap_or_else(|_| API_BASE.to_string());

    let url = format!("{api_base}/embeddings");

    let body = openai_build_embeddings_body(data, &self_.model);

    let mut request_data = RequestData::new(url, body);

    request_data.bearer_auth(api_key);
    if let Some(organization_id) = &self_.config.organization_id {
        request_data.header("OpenAI-Organization", organization_id);
    }

    Ok(request_data)
}

pub async fn openai_chat_completions(
    builder: RequestBuilder,
    _model: &Model,
) -> Result<ChatCompletionsOutput> {
    let res = builder.send().await?;
    let status = res.status();
    let data: Value = res.json().await?;
    if !status.is_success() {
        catch_error(&data, status.as_u16())?;
    }

    debug!("non-stream-data: {data}");
    openai_extract_chat_completions(&data)
}

pub async fn openai_chat_completions_streaming(
    builder: RequestBuilder,
    handler: &mut SseHandler,
    _model: &Model,
) -> Result<()> {
    let mut call_id = String::new();
    let mut function_name = String::new();
    let mut function_arguments = String::new();
    let mut function_id = String::new();
    let mut reasoning_state = 0;
    let handle = |message: SseMmessage| -> Result<bool> {
        if message.data == "[DONE]" {
            if !function_name.is_empty() {
                if function_arguments.is_empty() {
                    function_arguments = String::from("{}");
                }
                let arguments: Value = function_arguments.parse().with_context(|| {
                    format!("Tool call '{function_name}' have non-JSON arguments '{function_arguments}'")
                })?;
                handler.tool_call(ToolCall::new(
                    function_name.clone(),
                    arguments,
                    normalize_function_id(&function_id),
                ))?;
            }
            return Ok(true);
        }
        let data: Value = serde_json::from_str(&message.data)?;
        debug!("stream-data: {data}");
        if let Some(text) = data["choices"][0]["delta"]["content"]
            .as_str()
            .filter(|v| !v.is_empty())
        {
            if reasoning_state == 1 {
                handler.text("\n</think>\n\n")?;
                reasoning_state = 0;
            }
            handler.text(text)?;
        } else if let Some(text) = data["choices"][0]["delta"]["reasoning_content"]
            .as_str()
            .or_else(|| data["choices"][0]["delta"]["reasoning"].as_str())
            .filter(|v| !v.is_empty())
        {
            if reasoning_state == 0 {
                handler.text("<think>\n")?;
                reasoning_state = 1;
            }
            handler.text(text)?;
        }
        if let (Some(function), index, id) = (
            data["choices"][0]["delta"]["tool_calls"][0]["function"].as_object(),
            data["choices"][0]["delta"]["tool_calls"][0]["index"].as_u64(),
            data["choices"][0]["delta"]["tool_calls"][0]["id"]
                .as_str()
                .filter(|v| !v.is_empty()),
        ) {
            if reasoning_state == 1 {
                handler.text("\n</think>\n\n")?;
                reasoning_state = 0;
            }
            let maybe_call_id = format!("{}/{}", id.unwrap_or_default(), index.unwrap_or_default());
            if maybe_call_id != call_id && maybe_call_id.len() >= call_id.len() {
                if !function_name.is_empty() {
                    if function_arguments.is_empty() {
                        function_arguments = String::from("{}");
                    }
                    let arguments: Value = function_arguments.parse().with_context(|| {
                        format!("Tool call '{function_name}' have non-JSON arguments '{function_arguments}'")
                    })?;
                    handler.tool_call(ToolCall::new(
                        function_name.clone(),
                        arguments,
                        normalize_function_id(&function_id),
                    ))?;
                }
                function_name.clear();
                function_arguments.clear();
                function_id.clear();
                call_id = maybe_call_id;
            }
            if let Some(name) = function.get("name").and_then(|v| v.as_str()) {
                if name.starts_with(&function_name) {
                    function_name = name.to_string();
                } else {
                    function_name.push_str(name);
                }
            }
            if let Some(arguments) = function.get("arguments").and_then(|v| v.as_str()) {
                function_arguments.push_str(arguments);
            }
            if let Some(id) = id {
                function_id = id.to_string();
            }
        }
        Ok(false)
    };

    sse_stream(builder, handle).await
}

pub async fn openai_embeddings(
    builder: RequestBuilder,
    _model: &Model,
) -> Result<EmbeddingsOutput> {
    let res = builder.send().await?;
    let status = res.status();
    let data: Value = res.json().await?;
    if !status.is_success() {
        catch_error(&data, status.as_u16())?;
    }
    let res_body: EmbeddingsResBody =
        serde_json::from_value(data).context("Invalid embeddings data")?;
    let output = res_body.data.into_iter().map(|v| v.embedding).collect();
    Ok(output)
}

#[derive(Deserialize)]
struct EmbeddingsResBody {
    data: Vec<EmbeddingsResBodyEmbedding>,
}

#[derive(Deserialize)]
struct EmbeddingsResBodyEmbedding {
    embedding: Vec<f32>,
}

pub fn openai_build_chat_completions_body(data: ChatCompletionsData, model: &Model) -> Value {
    let ChatCompletionsData {
        messages,
        temperature,
        top_p,
        functions,
        stream,
        reasoning_effort,
    } = data;

    let messages_len = messages.len();
    let messages: Vec<Value> = messages
        .into_iter()
        .enumerate()
        .flat_map(|(i, message)| {
            let Message { role, content } = message;
            match content {
                MessageContent::ToolCalls(MessageContentToolCalls {
                    tool_results,
                    text: _,
                    sequence,
                }) => {
                    if !sequence {
                        let tool_calls: Vec<_> = tool_results
                            .iter()
                            .map(|tool_result| {
                                json!({
                                    "id": tool_result.call.id,
                                    "type": "function",
                                    "function": {
                                        "name": tool_result.call.name,
                                        "arguments": tool_result.call.arguments.to_string(),
                                    },
                                })
                            })
                            .collect();
                        let mut messages = vec![
                            json!({ "role": MessageRole::Assistant, "tool_calls": tool_calls }),
                        ];
                        for tool_result in tool_results {
                            messages.push(json!({
                                "role": "tool",
                                "content": tool_result.output.to_string(),
                                "tool_call_id": tool_result.call.id,
                            }));
                        }
                        messages
                    } else {
                        tool_results.into_iter().flat_map(|tool_result| {
                            vec![
                                json!({
                                    "role": MessageRole::Assistant,
                                    "tool_calls": [
                                        {
                                            "id": tool_result.call.id,
                                            "type": "function",
                                            "function": {
                                                "name": tool_result.call.name,
                                                "arguments": tool_result.call.arguments.to_string(),
                                            },
                                        }
                                    ]
                                }),
                                json!({
                                    "role": "tool",
                                    "content": tool_result.output.to_string(),
                                    "tool_call_id": tool_result.call.id,
                                })
                            ]

                        }).collect()
                    }
                }
                MessageContent::Text(text) if role.is_assistant() && i != messages_len - 1 => {
                    vec![json!({ "role": role, "content": strip_think_tag(&text) }
                    )]
                }
                _ => vec![json!({ "role": role, "content": content })],
            }
        })
        .collect();

    let mut body = json!({
        "model": &model.real_name(),
        "messages": messages,
    });

    if let Some(v) = model.max_tokens_param() {
        if model
            .patch()
            .and_then(|v| v.get("body").and_then(|v| v.get("max_tokens")))
            == Some(&Value::Null)
        {
            body["max_completion_tokens"] = v.into();
        } else {
            body["max_tokens"] = v.into();
        }
    }
    if let Some(v) = temperature {
        body["temperature"] = v.into();
    }
    if let Some(v) = top_p {
        body["top_p"] = v.into();
    }
    if stream {
        body["stream"] = true.into();
    }
    if let Some(effort) = reasoning_effort {
        body["reasoning_effort"] = effort.into();
    }
    if let Some(functions) = functions {
        body["tools"] = functions
            .iter()
            .map(|v| {
                json!({
                    "type": "function",
                    "function": v,
                })
            })
            .collect();
    }
    body
}

pub fn openai_build_embeddings_body(data: &EmbeddingsData, model: &Model) -> Value {
    json!({
        "input": data.texts,
        "model": model.real_name()
    })
}

pub fn openai_sanitize_chat_completions_body(body: &mut Value, model: &Model) {
    if body.get("messages").is_none() {
        return;
    }
    let model = Model::new(
        model.client_name(),
        body["model"].as_str().unwrap_or(model.real_name()),
    );
    let Some((major, minor)) = model.gpt_version() else {
        return;
    };
    if major < 5 {
        return;
    }

    // GPT-5 requires default sampling. Later GPT generations allow sampling
    // only with reasoning explicitly disabled; omission keeps API defaults.
    let reasoning_effort = body["reasoning_effort"]
        .as_str()
        .or_else(|| body["reasoning"]["effort"].as_str());
    if (major, minor) >= (5, 1) && reasoning_effort == Some("none") {
        return;
    }
    if let Some(body) = body.as_object_mut() {
        body.remove("temperature");
        body.remove("top_p");
    }
}

pub fn openai_extract_chat_completions(data: &Value) -> Result<ChatCompletionsOutput> {
    let text = data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default();

    let reasoning = data["choices"][0]["message"]["reasoning_content"]
        .as_str()
        .or_else(|| data["choices"][0]["message"]["reasoning"].as_str())
        .unwrap_or_default()
        .trim();

    let mut tool_calls = vec![];
    if let Some(calls) = data["choices"][0]["message"]["tool_calls"].as_array() {
        for call in calls {
            if let (Some(name), Some(arguments), Some(id)) = (
                call["function"]["name"].as_str(),
                call["function"]["arguments"].as_str(),
                call["id"].as_str(),
            ) {
                let arguments: Value = arguments.parse().with_context(|| {
                    format!("Tool call '{name}' have non-JSON arguments '{arguments}'")
                })?;
                tool_calls.push(ToolCall::new(
                    name.to_string(),
                    arguments,
                    Some(id.to_string()),
                ));
            }
        }
    };

    if text.is_empty() && tool_calls.is_empty() {
        bail!("Invalid response data: {data}");
    }
    let text = if !reasoning.is_empty() {
        format!("<think>\n{reasoning}\n</think>\n\n{text}")
    } else {
        text.to_string()
    };
    let output = ChatCompletionsOutput {
        text,
        tool_calls,
        id: data["id"].as_str().map(|v| v.to_string()),
        input_tokens: data["usage"]["prompt_tokens"].as_u64(),
        output_tokens: data["usage"]["completion_tokens"].as_u64(),
    };
    Ok(output)
}

fn normalize_function_id(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use parking_lot::RwLock;
    use std::sync::Arc;

    fn sampling_request_body(
        model: Model,
        stream: bool,
        reasoning_effort: Option<&str>,
        patch: Option<RequestPatch>,
    ) -> Value {
        let client = OpenAIClient {
            global_config: Arc::new(RwLock::new(Config::default())),
            config: OpenAIConfig {
                patch,
                ..Default::default()
            },
            model,
        };
        let data = ChatCompletionsData {
            messages: vec![Message::new(
                MessageRole::User,
                MessageContent::Text("hi".into()),
            )],
            temperature: Some(0.3),
            top_p: Some(0.8),
            functions: None,
            stream,
            reasoning_effort: reasoning_effort
                .and_then(|v| client.model.normalize_reasoning_effort(v.into())),
        };
        let body = openai_build_chat_completions_body(data, &client.model);
        let request = client
            .request_builder(
                &reqwest::Client::new(),
                RequestData::new("http://localhost/chat/completions", body),
            )
            .build()
            .unwrap();
        serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap()
    }

    #[test]
    fn test_reasoning_models_omit_sampling_in_final_requests() {
        for name in [
            "gpt-5",
            "gpt-5-mini",
            "gpt-5-nano-2025-08-07",
            "gpt-5.1",
            "gpt-5.4-mini",
            "gpt-5.6-sol",
            "gpt-6",
            "gpt-6-sol",
            "gpt-6.1-sol",
            "gpt-6-astra",
            "gpt-7",
            "openai/gpt-5",
            "openai/gpt-5:online",
            "openai/gpt-6-sol",
            "openai/gpt-6-sol:extended",
            "openai/gpt-6:extended",
        ] {
            for stream in [false, true] {
                for effort in [None, Some("medium"), Some("high")] {
                    let body = sampling_request_body(
                        Model::new("test", name),
                        stream,
                        effort,
                        None,
                    );
                    assert!(body.get("temperature").is_none(), "{name}: {body}");
                    assert!(body.get("top_p").is_none(), "{name}: {body}");
                    assert_eq!(body["model"], name);
                    assert_eq!(body["messages"][0]["content"], "hi");
                    assert_eq!(body.get("stream"), stream.then_some(&Value::Bool(true)));
                    assert_eq!(body["reasoning_effort"].as_str(), effort);
                }
            }
        }
    }

    #[test]
    fn test_sampling_is_preserved_when_supported() {
        for (name, effort) in [
            ("gpt-4o", None),
            ("gpt-4.1", Some("off")),
            ("deepseek-chat", None),
            ("claude-sonnet-5.5", Some("none")),
            ("gpt-5.1", Some("none")),
            ("gpt-5.4", Some("NONE")),
            ("gpt-6-sol", Some("none")),
            ("openai/gpt-6-luna", Some("none")),
        ] {
            let body = sampling_request_body(Model::new("test", name), false, effort, None);
            assert_eq!(body["temperature"], 0.3, "{name}: {body}");
            assert_eq!(body["top_p"], 0.8, "{name}: {body}");
            if effort.is_some_and(|v| v.eq_ignore_ascii_case("none"))
                && name.contains("gpt-")
            {
                assert_eq!(body["reasoning_effort"], "none");
            } else {
                assert!(body.get("reasoning_effort").is_none());
            }
        }
        let body = sampling_request_body(Model::new("test", "gpt-5"), false, Some("none"), None);
        assert!(body.get("temperature").is_none());
        assert!(body.get("top_p").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn test_sampling_validation_uses_real_name_and_final_patches() {
        let mut model = Model::new("openrouter", "my-alias");
        model.data_mut().real_name = Some("openai/gpt-6-sol".into());
        model.data_mut().patch = Some(json!({"body": {
            "temperature": 0.2,
            "top_p": 0.9,
            "reasoning_effort": "high"
        }}));
        let body = sampling_request_body(model.clone(), true, Some("none"), None);
        assert_eq!(body["model"], "openai/gpt-6-sol");
        assert_eq!(body["reasoning_effort"], "high");
        assert!(body.get("temperature").is_none());
        assert!(body.get("top_p").is_none());

        let patch = serde_json::from_value(json!({"chat_completions": {
            "my-alias": {"body": {"reasoning_effort": "none", "temperature": 0.4}}
        }}))
        .unwrap();
        let body = sampling_request_body(model.clone(), false, Some("high"), Some(patch));
        assert_eq!(body["reasoning_effort"], "none");
        assert_eq!(body["temperature"], 0.4);
        assert_eq!(body["top_p"], 0.9);

        let patch = serde_json::from_value(json!({"chat_completions": {
            "my-alias": {"body": {"temperature": 0.1, "top_p": 0.5}}
        }}))
        .unwrap();
        let body = sampling_request_body(model, false, None, Some(patch));
        assert!(body.get("temperature").is_none());
        assert!(body.get("top_p").is_none());
    }

    #[test]
    fn test_sampling_validation_does_not_inject_parameters_or_change_embeddings() {
        let model = Model::new("openai", "gpt-6-sol");
        let mut body = json!({"model": "gpt-6-sol", "messages": [], "stream": true});
        let original = body.clone();
        openai_sanitize_chat_completions_body(&mut body, &model);
        assert_eq!(body, original);

        let mut body = json!({"model": "gpt-6-sol", "input": ["hello"]});
        let original = body.clone();
        openai_sanitize_chat_completions_body(&mut body, &model);
        assert_eq!(body, original);
    }

    #[test]
    fn test_openai_reasoning_effort_in_body() {
        let model = Model::new("openai", "gpt-6-sol");
        let data = ChatCompletionsData {
            messages: vec![Message::new(
                MessageRole::User,
                MessageContent::Text("hi".into()),
            )],
            temperature: Some(0.7),
            top_p: None,
            functions: None,
            stream: false,
            reasoning_effort: Some("high".to_string()),
        };
        let body = openai_build_chat_completions_body(data, &model);
        assert_eq!(body["reasoning_effort"], "high");

        let data2 = ChatCompletionsData {
            messages: vec![Message::new(
                MessageRole::User,
                MessageContent::Text("hi".into()),
            )],
            temperature: Some(0.7),
            top_p: None,
            functions: None,
            stream: false,
            reasoning_effort: None,
        };
        let body2 = openai_build_chat_completions_body(data2, &model);
        assert!(body2.get("reasoning_effort").is_none());
    }
}
