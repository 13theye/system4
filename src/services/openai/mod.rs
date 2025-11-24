// src/services/open_ai.rs
//
// OpenAI REST API client

pub mod schema;

use crate::settings::OpenAIServiceConfig;
use schema::request::*;
use schema::response::*;

use reqwest::Client;
use std::error::Error;
use tokio::sync::{broadcast, mpsc};

pub struct OpenAIService {
    // LLM system prompt
    pub system_prompt: Option<String>,

    // Model name, url
    pub model: String,
    pub url: String,

    // Tokio runtime for async tasks
    runtime: Option<tokio::runtime::Runtime>,
    // Runtime shutdown channel:
    runtime_shutdown_tx: broadcast::Sender<()>,
    // Task channel
    task_tx: mpsc::Sender<Option<OpenAIResponse>>,
    task_rx: mpsc::Receiver<Option<OpenAIResponse>>,

    // Reqwest client:
    client: Client,
}

impl OpenAIService {
    pub fn new(config: &OpenAIServiceConfig) -> Self {
        let runtime = tokio::runtime::Runtime::new()
            .expect("Failed to start Tokio runtime for OpenAIService");

        let (shutdown_tx, _) = broadcast::channel(1);
        let (task_tx, task_rx) = mpsc::channel(1);

        Self {
            system_prompt: Some(config.system_prompt.to_owned()),
            model: config.model.to_owned(),
            url: config.url.to_owned(),
            runtime: Some(runtime),
            runtime_shutdown_tx: shutdown_tx,
            task_tx,
            task_rx,
            client: Client::new(),
        }
    }

    /// Send a request via OpenAI API
    pub fn send(&mut self, content: &str) -> Result<(), String> {
        // Clone the content, system prompt, model name, url, client
        let content = content.to_owned();
        let system_prompt = self.system_prompt.clone();
        let model = self.model.clone();
        let client = self.client.clone();
        let url = self.url.clone();

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
                    match generate_response(content, system_prompt, url, model, client).await {
                        Ok(response) => {
                            println!(
                                "OpenAIService: received successful response of length {}",
                                response.output.len()
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
    pub fn try_recv(&mut self) -> Option<OpenAIResponse> {
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

/// Sends a REST AP request via OpenAI API
async fn generate_response(
    content: String,
    prompt: Option<String>,
    url: String,
    model: String,
    client: Client,
) -> Result<OpenAIResponse, Box<dyn Error + Send + Sync>> {
    let reasoning = OpenAIReasoningConfig {
        effort: OpenAIReasoningEffort::Low,
    };

    // Map the plain model string from config to our enum
    let model = match model.as_str() {
        "openai/gpt-oss-20b" => OpenAIModelName::GptOss20b,
        other => {
            return Err(format!("Unsupported model name: {}", other).into());
        }
    };

    let instructions = prompt.unwrap_or("".to_owned());

    let request = OpenAIRequest {
        model,
        input: content,
        instructions,
        reasoning: Some(reasoning),
    };

    let request_raw =
        serde_json::to_string(&request).expect("OpenAIService task: failed to serialize request");

    // Debug: log request body before sending
    println!("OpenAIService: request body: {}", request_raw);

    // Ensure server treats body as JSON
    let response_http = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(request_raw)
        .send()
        .await?;

    let status = response_http.status();
    let response_raw = response_http.text().await?;

    if !status.is_success() {
        eprintln!(
            "OpenAIService: HTTP error {} with body: {}",
            status, response_raw
        );
        return Err(format!("HTTP {}: {}", status, response_raw).into());
    }

    // Debug: log raw response body before attempting JSON parse
    eprintln!("OpenAIService: raw response body: {}", response_raw);

    let response: OpenAIResponse = serde_json::from_str(&response_raw)?;

    // Treat either an explicit "completed" status or a missing status
    // (some backends omit it) as a successful, final response.
    if matches!(response.status.as_deref(), Some("completed") | None) {
        return Ok(response);
    }

    if let Some(err) = response.error {
        return Err(err.message.into());
    }

    Err(format!(
        "No response generated, unexpected status: {:?}",
        response.status
    )
    .into())
}
