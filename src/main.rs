use actix_web::{App, HttpServer, Responder, Result, post};
use actix_web_lab::sse::{self, Sse};
use falkordb::FalkorClientBuilder;
use falkordb::FalkorConnectionInfo;
use futures_util::StreamExt;
use genai::ModelIden;
use genai::resolver::AuthData;
use genai::resolver::AuthResolver;
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

// Macro for functions returning Result<String, Box<dyn Error>>
macro_rules! try_send_boxed {
    ($tx:expr, $progress:expr) => {
        match serde_json::to_string(&$progress) {
            Ok(json) => {
                let event = sse::Event::Data(sse::Data::new(json));
                if $tx.send(event).await.is_err() {
                    tracing::warn!("Client disconnected, stopping stream");
                    return Err("Client disconnected".into());
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize progress update: {}", e);
                return Err(format!("Serialization failed: {}", e).into());
            }
        }
    };
}

// Macro for functions returning String (returns empty string on error)
macro_rules! send_or_empty {
    ($tx:expr, $progress:expr) => {
        match serde_json::to_string(&$progress) {
            Ok(json) => {
                let event = sse::Event::Data(sse::Data::new(json));
                if $tx.send(event).await.is_err() {
                    tracing::warn!("Client disconnected, stopping stream");
                    return String::new();
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize progress update: {}", e);
                return String::new();
            }
        }
    };
}

mod chat;
mod error;
mod formatter;
mod schema;
mod template;

use chat::{ChatMessage, ChatRequest, ChatRole};
use error::ApiError;
use formatter::format_query_records;
use template::TemplateEngine;

use crate::schema::discovery::Schema;

#[derive(Serialize, Deserialize, ToSchema)]
struct HelloResponse {
    message: String,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
struct TextToCypherRequest {
    graph_name: String,
    chat_request: ChatRequest,
    model: String,
    key: Option<String>,
}

impl std::fmt::Debug for TextToCypherRequest {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("TextToCypherRequest");
        debug_struct
            .field("graph_name", &self.graph_name)
            .field("chat_request", &self.chat_request)
            .field("model", &self.model);

        if self.key.is_some() {
            debug_struct.field("key", &"***");
        }

        debug_struct.finish()
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
enum Progress {
    Status(String),
    Schema(String),
    CypherQuery(String),
    CypherResult(String),
    ModelOutputChunk(String),
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
    let request = req.into_inner();

    let client = request.key.as_ref().map_or_else(genai::Client::default, |key| {
        let key = key.clone(); // Clone the key for use in the closure
        let auth_resolver = AuthResolver::from_resolver_fn(
            move |model_iden: ModelIden| -> Result<Option<AuthData>, genai::resolver::Error> {
                let ModelIden {
                    adapter_kind,
                    model_name,
                } = model_iden;
                tracing::info!("Using custom auth provider for {adapter_kind} (model: {model_name})");

                // Use the provided key instead of reading from environment
                Ok(Some(AuthData::from_single(key.clone())))
            },
        );
        genai::Client::builder().with_auth_resolver(auth_resolver).build()
    });

    let service_target = client.resolve_service_target(&request.model).await.map_err(ApiError::from)?;

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

    send!(
        tx,
        Progress::Status(String::from("Generating Cypher query using schema ..."))
    );

    // Step 3: Generate chat request
    let genai_chat_request = generate_create_cypher_query_chat_request(&request.chat_request, &schema);

    // Step 4: Execute chat stream and process response
    let query = execute_chat(&client, &request.model, genai_chat_request, &tx).await;

    // Step 5: Execute the generated query
    if query.trim().is_empty() {
        tracing::warn!("No query generated from AI model");
        send!(tx, Progress::Error("No valid query was generated".to_string()));
        return;
    }

    let query = query.replace('\n', " ").replace("```", "").trim().to_string();

    send!(tx, Progress::CypherQuery(query.clone()));

    send!(tx, Progress::Status(String::from("Executing Cypher query...")));

    tracing::info!("Executing Cypher Query: {}", &query);

    let query_result = match execute_query(&query, &request.graph_name, &tx).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Query execution failed: {}", e);
            send!(tx, Progress::Error(format!("Query execution failed: {e}")));
            return;
        }
    };
    tracing::info!("Query executed successfully, result: {}", query_result);

    // Step 6: Send the final result
    send!(tx, Progress::CypherResult(query_result.clone()));

    send!(
        tx,
        Progress::Status(String::from(
            "Generating answer from chat history and Cypher output using AI model..."
        ))
    );

    let genai_chat_request: genai::chat::ChatRequest =
        generate_answer_chat_request(&request.chat_request, &query, &query_result);
    execute_chat_stream(&client, &request.model, genai_chat_request, &tx).await;
}

async fn execute_query(
    query: &str,
    graph_name: &str,
    tx: &mpsc::Sender<sse::Event>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let connection_info: FalkorConnectionInfo = "falkor://127.0.0.1:6379"
        .try_into()
        .map_err(|e| format!("Invalid connection info: {e}"))?;

    let client = FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let graph_name = graph_name.to_string();
    let query = query.to_string();

    // Run the FalkorDB operations in a blocking context
    let result = tokio::task::spawn_blocking(move || execute_query_blocking(&client, &graph_name, &query))
        .await
        .map_err(|e| format!("Failed to execute blocking task: {e}"))?;

    let formatted_result = match result {
        Ok(records) => format_query_records(&records),
        Err(e) => {
            let error_msg = format!("Query execution failed: {e}");
            try_send_boxed!(tx, Progress::Error(error_msg.clone()));
            return Err(error_msg.into());
        }
    };

    Ok(formatted_result)
}

fn execute_query_blocking(
    client: &falkordb::FalkorAsyncClient,
    graph_name: &str,
    query: &str,
) -> Result<Vec<Vec<falkordb::FalkorValue>>, Box<dyn std::error::Error + Send + Sync>> {
    // Create a new Tokio runtime for this blocking operation
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;

    rt.block_on(async {
        let mut graph = client.select_graph(graph_name);
        let query_result = graph
            .ro_query(query)
            .execute()
            .await
            .map_err(|e| format!("Query execution failed: {e}"))?;

        let mut records = Vec::new();
        for record in query_result.data {
            records.push(record);
        }
        Ok(records)
    })
}

fn generate_create_cypher_query_chat_request(
    chat_request: &ChatRequest,
    ontology: &str,
) -> genai::chat::ChatRequest {
    let mut chat_req = genai::chat::ChatRequest::default();
    for (index, message) in chat_request.messages.iter().enumerate() {
        let is_last_user_message = index == chat_request.messages.len() - 1 && message.role == ChatRole::User;

        let genai_message = match message.role {
            ChatRole::User => {
                if is_last_user_message {
                    // Special processing for the last user message
                    let processed_content = process_last_user_message(&message.content);
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

fn generate_answer_chat_request(
    chat_request: &ChatRequest,
    cypher_query: &str,
    cypher_result: &str,
) -> genai::chat::ChatRequest {
    let mut chat_req = genai::chat::ChatRequest::default();
    for (index, message) in chat_request.messages.iter().enumerate() {
        let is_last_user_message = index == chat_request.messages.len() - 1 && message.role == ChatRole::User;

        let genai_message = match message.role {
            ChatRole::User => {
                if is_last_user_message {
                    // Special processing for the last user message
                    let processed_content = process_last_request_prompt(&message.content, cypher_query, cypher_result);
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

    // Pretty print the chat request as JSON for logging
    if let Ok(pretty_json) = serde_json::to_string_pretty(&chat_req) {
        tracing::info!("Generated genai chat request:\n{}", pretty_json);
    } else {
        tracing::info!("Generated genai chat request: {:?}", chat_req);
    }
    chat_req
}

fn process_last_request_prompt(
    content: &str,
    cypher_query: &str,
    cypher_result: &str,
) -> String {
    TemplateEngine::render_last_request_prompt(content, cypher_query, cypher_result).unwrap_or_else(|e| {
        tracing::error!("Failed to load last_request_prompt template: {}", e);
        format!("Generate an answer for: {content}")
    })
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

fn process_last_user_message(question: &str) -> String {
    TemplateEngine::render_user_prompt(question).unwrap_or_else(|e| {
        tracing::error!("Failed to load user prompt template: {}", e);
        format!("Generate an OpenCypher statement for: {question}")
    })
}

#[allow(clippy::cognitive_complexity)]
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

async fn execute_chat(
    client: &genai::Client,
    model: &str,
    genai_chat_request: genai::chat::ChatRequest,
    tx: &mpsc::Sender<sse::Event>,
) -> String {
    // Make the actual request to the model
    let chat_response = match client.exec_chat(model, genai_chat_request, None).await {
        Ok(response) => response,
        Err(e) => {
            let error_update = Progress::Error(format!("Chat request failed: {e}"));
            send_or_empty!(tx, error_update);
            return String::from("NO ANSWER");
        }
    };
    chat_response
        .content_text_into_string()
        .unwrap_or_else(|| String::from("NO ANSWER"))
}

async fn execute_chat_stream(
    client: &genai::Client,
    model: &str,
    genai_chat_request: genai::chat::ChatRequest,
    tx: &mpsc::Sender<sse::Event>,
) -> String {
    // Make the actual request to the model
    let chat_response = match client.exec_chat_stream(model, genai_chat_request, None).await {
        Ok(response) => response,
        Err(e) => {
            let error_update = Progress::Error(format!("Chat request failed: {e}"));
            send_or_empty!(tx, error_update);
            return String::new();
        }
    };

    process_chat_stream(chat_response, tx).await
}

#[allow(clippy::cognitive_complexity)]
async fn process_chat_stream(
    chat_response: genai::chat::ChatStreamResponse,
    tx: &mpsc::Sender<sse::Event>,
) -> String {
    let mut answer = String::new();

    // Extract the response stream
    let mut stream = chat_response.stream;
    while let Some(Ok(stream_event)) = stream.next().await {
        match stream_event {
            genai::chat::ChatStreamEvent::Start => {}
            genai::chat::ChatStreamEvent::Chunk(chunk) => {
                answer.push_str(&chunk.content);
                send_or_empty!(tx, Progress::ModelOutputChunk(chunk.content));
            }
            genai::chat::ChatStreamEvent::ReasoningChunk(_chunk) => {}
            genai::chat::ChatStreamEvent::End(_end_event) => {}
        }
    }

    tracing::info!("Final answer: {}", answer);
    send_or_empty!(tx, Progress::Result(answer.clone()));
    answer
}
