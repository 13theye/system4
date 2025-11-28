// Primary OpenAI service module.
//
// This module provides a fire-and-poll synchronous façade backed by a Tokio
// runtime and reqwest.

// src/services/open_ai.rs
//
// OpenAI REST API client

pub mod schema;

use crate::{
    services::openai::schema::request::{ReasoningConfigExt, TextConfigExt},
    settings::OpenAIServiceConfig,
};

use async_openai::{
    config::OpenAIConfig,
    types::responses::{self as openai_response},
    Client,
};
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct OpenAIService {
    // LLM system prompt
    pub system_prompt: Option<String>,

    // Response Schema description
    pub schema_description: Option<String>,

    // Model name, url, schema
    pub model: String,

    // Strict adherence to OpenAI API request schema
    // Set to false when using LMStudio
    pub strict_request_object_adherence: bool,

    // Tokio runtime for async tasks
    runtime: Option<tokio::runtime::Runtime>,
    // Runtime shutdown channel:
    runtime_shutdown_tx: broadcast::Sender<()>,
    // Task channel
    task_tx: mpsc::Sender<Option<openai_response::Response>>,
    task_rx: mpsc::Receiver<Option<openai_response::Response>>,

    // Reqwest client:
    client: Client<OpenAIConfig>,
}

impl OpenAIService {
    pub fn new(config: &OpenAIServiceConfig) -> Self {
        let runtime = tokio::runtime::Runtime::new()
            .expect("Failed to start Tokio runtime for OpenAIService");

        let (shutdown_tx, _) = broadcast::channel(1);
        let (task_tx, task_rx) = mpsc::channel(1);

        let openai_config = OpenAIConfig::new()
            .with_api_key(config.api_key.to_owned().unwrap_or_default())
            .with_api_base(config.url.to_owned());

        Self {
            system_prompt: Some(config.system_prompt.to_owned()),
            schema_description: config.schema_description.clone(),
            model: config.model.to_owned(),
            strict_request_object_adherence: config.strict_request_object_adherence,
            runtime: Some(runtime),
            runtime_shutdown_tx: shutdown_tx,
            task_tx,
            task_rx,
            client: Client::with_config(openai_config),
        }
    }

    /// Send a request via OpenAI API
    pub fn send(&mut self, content: String) -> Result<(), String> {
        // Clone the content, system prompt, schema description, model name, url, client
        let content = content.to_owned();
        let system_prompt = self.system_prompt.clone();
        let schema_description = self.schema_description.clone();
        let model = self.model.clone();
        let client = self.client.clone();
        let strict_object_adherence = self.strict_request_object_adherence;

        if let Some(runtime) = &self.runtime {
            let tx = self.task_tx.clone();
            let mut shutdown_rx = self.runtime_shutdown_tx.subscribe();

            runtime.spawn(async move {
                println!("OpenAIService: Send task created");

                // Start listening for shutdown signal
                let shutdown = async {
                    let _ = shutdown_rx.recv().await;
                };

                let task = async {
                    match generate_response(
                        content,
                        system_prompt,
                        schema_description,
                        model,
                        &client,
                        strict_object_adherence,
                    )
                    .await
                    {
                        Ok(response) => {
                            println!(
                                "OpenAIService: Received successful response with id: {}",
                                response.id
                            );

                            // Pass message back to OpenAIService main task
                            let _ = tx.send(Some(response)).await;
                        }
                        Err(e) => {
                            eprintln!("OpenAIService: API error: {}", e);
                            if let Some(source) = e.source() {
                                eprintln!("OpenAIService: error source: {}", source);
                            }
                        }
                    }
                };

                tokio::select! {
                    _ = shutdown => {
                        println!("...OpenAIService received shutdown signal");
                    }
                    _ = task => {
                        println!("...OpenAIService task completed normally");
                    }
                }
            });
        }

        Ok(())
    }

    /// Try to retrieve a completed response, if any.
    pub fn try_recv(&mut self) -> Option<openai_response::Response> {
        match self.task_rx.try_recv() {
            Ok(Some(response)) => Some(response),
            Ok(None) | Err(_) => None,
        }
    }

    /// Gracefully shuts down the Tokio runtime
    pub fn shutdown(&mut self) {
        println!("OpenAIService: shutting down");

        // Signal all tasks to terminate
        let _ = self.runtime_shutdown_tx.send(());

        // Take ownership of runtime
        if let Some(runtime) = self.runtime.take() {
            // Shut down runtime from a separate thread to avoid blocking
            std::thread::spawn(move || {
                println!(".....Shutting down OpenAIService runtime in separate thread...");
                runtime.shutdown_timeout(std::time::Duration::from_secs(1));
            })
            .join()
            .ok();
            println!(".....OpenAIService runtime shutdown successfully");
        }
    }
}

impl Drop for OpenAIService {
    fn drop(&mut self) {
        println!("OpenAIService: dropping");
        self.shutdown();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Sends a REST API request via OpenAI Responses using openai-api-rs types
#[allow(clippy::too_many_arguments)]
async fn generate_response(
    content: String,
    prompt: Option<String>,
    schema_description: Option<String>,
    model: String,
    client: &Client<OpenAIConfig>,
    // strict adherence to OpenAPI Responses API
    strict: bool,
) -> Result<openai_response::Response, Box<dyn Error + Send + Sync>> {
    let text_config = openai_response::TextConfig::generate_for_system4_schema(schema_description);
    let reasoning_config = openai_response::ReasoningConfig::generate();

    let request = if !strict {
        // workaround to append the instructions to the input because the
        // "instructions" field isn't working in LMStudio
        let text = serde_json::to_string(&text_config).unwrap_or_default();
        let content = if let Some(prompt) = prompt {
            format!(
                "input : {}\ninstructions : {}\n text: {}",
                content, prompt, text
            )
        } else {
            format!("input : {}\n text: {}", content, text)
        };

        openai_response::CreateResponseArgs::default()
            .model(model)
            .input(content)
            .reasoning(reasoning_config)
            .build()?
    } else {
        // strict adherence to OpenAPI Responses API: use all relevant fields
        openai_response::CreateResponseArgs::default()
            .model(model)
            .input(content)
            .instructions(prompt.unwrap_or_default())
            .reasoning(reasoning_config)
            .text(text_config)
            .build()?
    };

    println!("OpenAIService: Response Request object:\n{:#?}", request);

    let response = client.responses().create(request).await?;

    Ok(response)
}

/*
/// Sends a REST API request via OpenAI Responses using openai-api-rs types
#[allow(clippy::too_many_arguments)]
async fn generate_response(
    content: String,
    prompt: Option<String>,
    schema_description: Option<String>,
    url: String,
    model: String,
    client: Client<OpenAIConfig>,
    api_key: Option<String>,
    strict: bool,
) -> Result<ResponseObject, Box<dyn Error + Send + Sync>> {
    // Build CreateResponseRequest from simple string fields.
    let mut request = CreateResponse::new();
    request.model = Some(model);

    let text_object = TextObject::new(schema_description);

    // Workaround to append the instructions to the input because the
    // "instructions" field is not working in LMStudio
    if !strict {
        let text = serde_json::to_string(&text_object).unwrap_or_default();
        let content = if let Some(prompt) = prompt {
            format!(
                "input : {}\ninstructions : {}\n text: {}",
                content, prompt, text
            )
        } else {
            format!("input: {}\ntext: {}", content, text)
        };

        request.input = Some(json!(content));
    } else {
        // Strictly adhere to OpenAI Response Request object schema
        let text = Some(json!(text_object));

        request.input = Some(json!(content));
        if let Some(prompt) = prompt {
            request.instructions = Some(serde_json::to_string(&prompt).unwrap_or_default());
        }
        request.text = text;
    }

    // Optional: request low-effort reasoning, matching previous behavior.
    request.reasoning = Some(json!({ "effort": "low" }));

    println!("OpenAIService: Response Request object:\n{:#?}", request);

    let request_raw = serde_json::to_string_pretty(&request)
        .expect("OpenAIService task: failed to serialize request");

    println!("OpenAIService: sending response request");

    // Ensure server treats body as JSON
    let response_http = if let Some(api_key) = api_key {
        client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .body(request_raw)
            .send()
            .await?
    } else {
        client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(request_raw)
            .send()
            .await?
    };

    let status = response_http.status();
    let response_raw = response_http.text().await?;

    if !status.is_success() {
        eprintln!(
            "OpenAIService: HTTP error {} with body: {}",
            status, response_raw
        );
        return Err(format!("HTTP {}: {}", status, response_raw).into());
    }

    let response: ResponseObject = serde_json::from_str(&response_raw)?;

    // Treat either an explicit "completed" status or a missing status
    // (some backends omit it) as a successful, final response.
    if matches!(response.status.as_deref(), Some("completed") | None) {
        // If there is an error object, surface it as an Err.
        if let Some(err) = &response.error {
            return Err(format!("OpenAI error: {}", err).into());
        }
        return Ok(response);
    }

    if let Some(err) = &response.error {
        return Err(format!("OpenAI error: {}", err).into());
    }

    Err(format!(
        "No response generated, unexpected status: {:?}",
        response.status
    )
    .into())
}
*/
