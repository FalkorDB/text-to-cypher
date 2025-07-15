use actix_web::{App, HttpServer, Responder, Result, post};
use actix_web_lab::sse::{self, Sse};
use falkordb::FalkorClientBuilder;
use falkordb::FalkorConnectionInfo;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing_subscriber::fmt;
use utoipa::OpenApi;
use utoipa::ToSchema;
use utoipa_swagger_ui::SwaggerUi;

// Macro for functions returning ()
macro_rules! send {
    ($tx:expr, $progress:expr) => {
        match serde_json::to_string(&$progress) {
            Ok(json) => {
                let event = sse::Event::Data(sse::Data::new(json));
                if $tx.send(event).await.is_err() {
                    tracing::warn!("Client disconnected, stopping stream");
                    return;
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize progress update: {}", e);
                return;
            }
        }
    };
}

// Macro for functions returning Result<T, ()> - same name, different internal marker
macro_rules! try_send {
    ($tx:expr, $progress:expr) => {
        match serde_json::to_string(&$progress) {
            Ok(json) => {
                let event = sse::Event::Data(sse::Data::new(json));
                if $tx.send(event).await.is_err() {
                    tracing::warn!("Client disconnected, stopping stream");
                    return Err(());
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize progress update: {}", e);
                return Err(());
            }
        }
    };
}

mod chat;
mod error;
mod schema;
mod template;

use chat::{ChatMessage, ChatRequest, ChatRole};
use error::ApiError;
use template::TemplateEngine;

use crate::schema::discovery::Schema;

#[derive(Serialize, Deserialize, ToSchema)]
struct HelloResponse {
    message: String,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
struct TextToCypherRequest {
    graph_name: String,
    chat_request: ChatRequest,
    model: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
enum Progress {
    Status(String),
    Schema(String),
    Cypher(String),
    ModelOutputChunk(String, String),
    Result(String),
    Error(String),
}

#[utoipa::path(
    post,
    path = "/text_to_cypher",
    request_body = TextToCypherRequest,
    responses(
        (status = 200, description = "Stream text to Cypher conversion progress", content_type = "text/event-stream")
    )
)]
#[post("/text_to_cypher")]
async fn text_to_cypher(req: actix_web::web::Json<TextToCypherRequest>) -> Result<impl Responder, actix_web::Error> {
    let mut request = req.into_inner();

    // Initialize the client outside the spawn
    let client = genai::Client::default();
    let service_target = client.resolve_service_target(&request.model).await.map_err(ApiError::from)?;

    tracing::info!("Resolved service target: {:?}", service_target.model);

    let (tx, rx) = mpsc::channel(100);

    tokio::spawn(async move {
        process_text_to_cypher_request(request, client, service_target, tx).await;
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(Ok::<_, actix_web::Error>);

    Ok(Sse::from_stream(stream))
}

async fn process_text_to_cypher_request(
    request: TextToCypherRequest,
    client: genai::Client,
    service_target: genai::ServiceTarget,
    tx: mpsc::Sender<sse::Event>,
) {
    tracing::info!("Processing text to Cypher request: {request:?}");

    // Step 1: Send processing status
    send_processing_status(&request, &service_target, &tx).await;

    // Step 2: Discover schema
    let Ok(schema) = discover_and_send_schema(&request.graph_name, &tx).await else {
        return;
    };

    // Step 3: Generate chat request
    let genai_chat_request = generate_chat_request(&request.chat_request, &schema);

    // Step 4: Execute chat stream and process response
    execute_chat_stream(&client, &request.model, genai_chat_request, &tx).await;
}

fn generate_chat_request(
    chat_request: &ChatRequest,
    ontology: &str,
) -> genai::chat::ChatRequest {
    let mut chat_req = genai::chat::ChatRequest::default();

    // Add user messages with special processing for the last user message
    for (index, message) in chat_request.messages.iter().enumerate() {
        let is_last_user_message = index == chat_request.messages.len() - 1 && message.role == ChatRole::User;

        let genai_message = match message.role {
            ChatRole::User => {
                if is_last_user_message {
                    // Special processing for the last user message
                    let processed_content = process_last_user_message(&message.content, ontology);
                    genai::chat::ChatMessage::user(processed_content)
                } else {
                    genai::chat::ChatMessage::user(message.content.clone())
                }
            }
            ChatRole::Assistant => genai::chat::ChatMessage::assistant(message.content.clone()),
            ChatRole::System => genai::chat::ChatMessage::system(message.content.clone()),
        };

        chat_req = chat_req.append_message(genai_message);
    }

    chat_req = chat_req.with_system(TemplateEngine::render_system_prompt(ontology).unwrap_or_else(|e| {
        tracing::error!("Failed to load system prompt template: {}", e);
        format!("Generate OpenCypher statements using this ontology: {ontology}")
    }));

    // Pretty print the chat request as JSON for logging
    if let Ok(pretty_json) = serde_json::to_string_pretty(&chat_req) {
        tracing::info!("Generated genai chat request:\n{}", pretty_json);
    } else {
        tracing::info!("Generated genai chat request: {:?}", chat_req);
    }
    chat_req
}

#[derive(OpenApi)]
#[openapi(
    paths(text_to_cypher),
    components(schemas(
        TextToCypherRequest,
        Progress,
        ChatRequest,
        ChatMessage,
        ChatRole,
        error::ErrorResponse
    ))
)]
struct ApiDoc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    fmt().with_max_level(tracing::Level::INFO).init();

    tracing::info!("Starting server at http://localhost:8080/swagger-ui/");

    // Start the HTTP server with Swagger UI at /swagger-ui/
    // OpenAPI documentation will be available at /api-doc/openapi.json
    // Swagger UI will be accessible at:
    // http://localhost:8080/swagger-ui/

    HttpServer::new(|| {
        App::new()
            .service(text_to_cypher)
            .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-doc/openapi.json", ApiDoc::openapi()))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

async fn discover_graph_schema(graph_name: &str) -> Schema {
    let connection_info: FalkorConnectionInfo = "falkor://127.0.0.1:6379".try_into().expect("Invalid connection info");

    let client = FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
        .expect("Failed to build client");

    // Select the specified graph
    let mut graph = client.select_graph(graph_name);
    let schema = Schema::discover_from_graph(&mut graph, 100)
        .await
        .expect("Failed to discover schema from graph");

    // Print the discovered schema
    tracing::info!("Discovered schema: {schema}");
    schema
}

fn process_last_user_message(
    question: &str,
    _ontology: &str,
) -> String {
    TemplateEngine::render_user_prompt(question).unwrap_or_else(|e| {
        tracing::error!("Failed to load user prompt template: {}", e);
        format!("Generate an OpenCypher statement for: {question}")
    })
}

async fn discover_and_send_schema(
    graph_name: &str,
    tx: &mpsc::Sender<sse::Event>,
) -> Result<String, ()> {
    try_send!(
        tx,
        Progress::Status(format!("Discovering schema for graph: {graph_name}"))
    );

    let schema = discover_graph_schema(graph_name).await;

    // Serialize and handle errors inline
    let Ok(json_schema) = serde_json::to_string(&schema) else {
        tracing::error!("Failed to serialize schema to JSON");
        try_send!(tx, Progress::Error("Failed to serialize schema".to_string()));
        return Err(());
    };

    tracing::info!("Discovered schema: {}", json_schema);
    try_send!(tx, Progress::Schema(json_schema.clone()));
    Ok(json_schema)
}

async fn send_processing_status(
    request: &TextToCypherRequest,
    service_target: &genai::ServiceTarget,
    tx: &mpsc::Sender<sse::Event>,
) {
    let adapter_kind = service_target.model.adapter_kind;
    send!(
        tx,
        Progress::Status(format!(
            "Processing query for graph: {} using model: {} ({:?})",
            request.graph_name, request.model, adapter_kind
        ))
    );
}

async fn execute_chat_stream(
    client: &genai::Client,
    model: &str,
    genai_chat_request: genai::chat::ChatRequest,
    tx: &mpsc::Sender<sse::Event>,
) {
    // Make the actual request to the model
    let chat_response = match client.exec_chat_stream(model, genai_chat_request, None).await {
        Ok(response) => response,
        Err(e) => {
            let error_update = Progress::Error(format!("Chat request failed: {e}"));
            send!(tx, error_update);
            return;
        }
    };

    process_chat_stream(chat_response, tx).await;
}

async fn process_chat_stream(
    chat_response: genai::chat::ChatStreamResponse,
    tx: &mpsc::Sender<sse::Event>,
) {
    let mut answer = String::new();

    // Extract the response stream
    let mut stream = chat_response.stream;
    while let Some(Ok(stream_event)) = stream.next().await {
        match stream_event {
            genai::chat::ChatStreamEvent::Start => {
                send!(
                    tx,
                    Progress::ModelOutputChunk("\n-- ChatStreamEvent::Start\n".to_string(), String::new())
                );
            }
            genai::chat::ChatStreamEvent::Chunk(chunk) => {
                answer.push_str(&chunk.content);
                send!(
                    tx,
                    Progress::ModelOutputChunk("\n-- ChatStreamEvent::Chunk:\n".to_string(), chunk.content)
                );
            }
            genai::chat::ChatStreamEvent::ReasoningChunk(chunk) => {
                send!(
                    tx,
                    Progress::ModelOutputChunk("\n-- ChatStreamEvent::ReasoningChunk:\n".to_string(), chunk.content)
                );
            }
            genai::chat::ChatStreamEvent::End(end_event) => {
                send!(
                    tx,
                    Progress::ModelOutputChunk(format!("\n-- ChatStreamEvent::End {end_event:?}\n"), String::new())
                );
            }
        }
    }

    tracing::info!("Final answer: {}", answer);
    send!(tx, Progress::Result(answer));
}
