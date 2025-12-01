// Primary OpenAI service module.
//
// This module provides a fire-and-poll synchronous façade backed by a Tokio
// runtime and reqwest.

// src/services/open_ai.rs
//
// OpenAI REST API client

pub mod schema;

use crate::{
    services::openai::schema::{
        request::{ReasoningParamExt, ResponseTextParamExt},
        response::ResponseObject,
    },
    settings::OpenAIServiceConfig,
};

use async_openai::{
    config::OpenAIConfig,
    types::responses::{self},
    Client,
};
use futures::StreamExt;
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
    task_tx: mpsc::Sender<Option<ResponseObject>>,
    task_rx: mpsc::Receiver<Option<ResponseObject>>,

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
        let streaming = false;

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
                    // Generate the response request
                    let request = generate_request(
                        content,
                        system_prompt,
                        schema_description,
                        model,
                        strict_object_adherence,
                        streaming,
                    );

                    let request = match request {
                        Ok(request) => request,
                        Err(e) => {
                            eprintln!("OpenAIService: Failed to generate request: {}", e);
                            return;
                        }
                    };

                    // Send the request
                    match send_response_request(request, &client).await {
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

                // Wait for shutdown signal or task completion
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

    pub fn stream(&mut self, content: String) -> Result<(), String> {
        use responses::ResponseStreamEvent;

        // Clone the content, system prompt, schema description, model name, url, client
        let content = content.to_owned();
        let system_prompt = self.system_prompt.clone();
        let schema_description = self.schema_description.clone();
        let model = self.model.clone();
        let client = self.client.clone();
        let strict_object_adherence = self.strict_request_object_adherence;
        let streaming = true;

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
                    // Generate the response request
                    let request = generate_request(
                        content,
                        system_prompt,
                        schema_description,
                        model,
                        strict_object_adherence,
                        streaming,
                    );

                    let request = match request {
                        Ok(request) => request,
                        Err(e) => {
                            eprintln!("OpenAIService: Failed to generate request: {}", e);
                            return;
                        }
                    };

                    // Send the stream request
                    let stream = send_stream_request(request, &client).await;

                    let Ok(mut stream) = stream else {
                        return;
                    };

                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(ResponseStreamEvent::ResponseCreated(event)) => {
                                println!("Response created with id: {}", event.response.id);
                                break;
                            }
                            Ok(ResponseStreamEvent::ResponseOutputTextDelta(event)) => {
                                println!("Response delta text: {}", event.delta);
                                break;
                            }
                            _ => {}
                        }
                    }
                };

                // Wait for shutdown signal or task completion
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
    pub fn try_recv(&mut self) -> Option<ResponseObject> {
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
fn generate_request(
    content: String,
    prompt: Option<String>,
    schema_description: Option<String>,
    model: String,
    // strict adherence to OpenAPI Responses API
    strict: bool,
    streaming: bool,
) -> Result<responses::CreateResponse, Box<dyn Error + Send + Sync>> {
    let text_config = responses::ResponseTextParam::generate_for_system4_schema(schema_description);
    let reasoning_config = responses::Reasoning::generate();

    let mut request = if !strict {
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

        responses::CreateResponseArgs::default()
            .model(model)
            .input(content)
            .reasoning(reasoning_config)
            .build()?
    } else {
        // strict adherence to OpenAPI Responses API: use all relevant fields
        responses::CreateResponseArgs::default()
            .model(model)
            .input(content)
            .instructions(prompt.unwrap_or_default())
            .reasoning(reasoning_config)
            .text(text_config)
            .build()?
    };

    // Set streaming flag if true
    if streaming {
        request.stream = Some(true);
    }

    println!("OpenAIService: Response Request object:\n{:#?}", request);

    Ok(request)
}

async fn send_response_request(
    request: responses::CreateResponse,
    client: &Client<OpenAIConfig>,
) -> Result<ResponseObject, Box<dyn Error + Send + Sync>> {
    let response: ResponseObject = client.responses().create_byot(request).await?;
    Ok(response)
}

async fn send_stream_request(
    request: responses::CreateResponse,
    client: &Client<OpenAIConfig>,
) -> Result<responses::ResponseStream, Box<dyn Error + Send + Sync>> {
    let stream: responses::ResponseStream = client.responses().create_stream_byot(request).await?;
    Ok(stream)
}
