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

mod chat;
mod error;
mod schema;

use chat::{ChatMessage, ChatRequest, ChatRole};
use error::ApiError;

use crate::schema::schema::Schema;

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
struct ProgressUpdate {
    status: String,
    message: String,
    result: Option<String>,
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
async fn text_to_cypher(
    req: actix_web::web::Json<TextToCypherRequest>
) -> Result<impl Responder, actix_web::Error> {
    let mut request = req.into_inner();

    request.model = "llama3.2".to_string(); // Default model, can be overridden by the request

    // Initialize the client outside the spawn
    let client = genai::Client::default();
    let service_target = client
        .resolve_service_target(&request.model)
        .await
        .map_err(ApiError::from)?;

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
    let adapter_kind = service_target.model.adapter_kind;
    // Send initial status
    let initial_update = ProgressUpdate {
        status: "pending".to_string(),
        message: "Job submitted, starting processing...".to_string(),
        result: None,
    };

    let event = sse::Event::Data(sse::Data::new(
        serde_json::to_string(&initial_update).unwrap(),
    ));

    if tx.send(event).await.is_err() {
        return;
    }

    // Send processing status
    let processing_update = ProgressUpdate {
        status: "processing".to_string(),
        message: format!(
            "Processing query for graph: {} using model: {} ({:?})",
            request.graph_name, request.model, adapter_kind
        ),
        result: None,
    };

    let event = sse::Event::Data(sse::Data::new(
        serde_json::to_string(&processing_update).unwrap(),
    ));

    if tx.send(event).await.is_err() {
        return;
    }

    // Convert our ChatRequest to genai::chat::ChatRequest
    let genai_chat_request: genai::chat::ChatRequest = request.chat_request.into();

    // Make the actual request to the model
    let chat_response = match client
        .exec_chat_stream(&request.model, genai_chat_request, None)
        .await
    {
        Ok(response) => response,
        Err(e) => {
            let error_update = ProgressUpdate {
                status: "failed".to_string(),
                message: format!("Chat request failed: {e}"),
                result: None,
            };

            let event = sse::Event::Data(sse::Data::new(
                serde_json::to_string(&error_update).unwrap(),
            ));

            let _ = tx.send(event).await;
            return;
        }
    };

    let mut answer = String::new();

    // Extract the response stream
    let mut stream = chat_response.stream;
    while let Some(Ok(stream_event)) = stream.next().await {
        let (event_info, content) = match stream_event {
            genai::chat::ChatStreamEvent::Start => {
                (Some("\n-- ChatStreamEvent::Start\n".to_string()), None)
            }
            genai::chat::ChatStreamEvent::Chunk(chunk) => {
                answer.push_str(&chunk.content);
                (
                    Some("\n-- ChatStreamEvent::Chunk:\n".to_string()),
                    Some(chunk.content),
                )
            }
            genai::chat::ChatStreamEvent::ReasoningChunk(chunk) => (
                Some("\n-- ChatStreamEvent::ReasoningChunk:\n".to_string()),
                Some(chunk.content),
            ),
            genai::chat::ChatStreamEvent::End(end_event) => (
                Some(format!("\n-- ChatStreamEvent::End {end_event:?}\n")),
                None,
            ),
        };

        if let Some(event_info) = event_info {
            let update = ProgressUpdate {
                status: "processing".to_string(),
                message: event_info,
                result: content,
            };

            let event = sse::Event::Data(sse::Data::new(serde_json::to_string(&update).unwrap()));

            if tx.send(event).await.is_err() {
                return;
            }
        }
    }

    // Send final summary
    let summary = ProgressUpdate {
        status: "completed".to_string(),
        message: "Processing completed successfully.".to_string(),
        result: Some(answer),
    };

    let event = sse::Event::Data(sse::Data::new(serde_json::to_string(&summary).unwrap()));

    let _ = tx.send(event).await;
}

#[derive(OpenApi)]
#[openapi(
    paths(text_to_cypher),
    components(schemas(
        TextToCypherRequest,
        ProgressUpdate,
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

    let schema = create_schema().await;
    // Print the discovered schema
    tracing::info!("Discovered schema: {}", schema);

    // Print the schema as JSON
    match serde_json::to_string_pretty(&schema) {
        Ok(json) => tracing::info!("Schema as JSON: \n{}", json),
        Err(e) => tracing::error!("Failed to serialize schema to JSON: {}", e),
    }

    tracing::info!("Starting server at http://localhost:8080/swagger-ui/");

    // Start the HTTP server with Swagger UI at /swagger-ui/
    // OpenAPI documentation will be available at /api-doc/openapi.json
    // Swagger UI will be accessible at:
    // http://localhost:8080/swagger-ui/

    HttpServer::new(|| {
        App::new().service(text_to_cypher).service(
            SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-doc/openapi.json", ApiDoc::openapi()),
        )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

async fn create_schema() -> Schema {
    let connection_info: FalkorConnectionInfo = "falkor://127.0.0.1:6379"
        .try_into()
        .expect("Invalid connection info");

    let client = FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
        .expect("Failed to build client");

    // Select the social graph
    let mut graph = client.select_graph("social");
    let schema = Schema::discover_from_graph(&mut graph, 100)
        .await
        .expect("Failed to discover schema from graph");

    // Print the discovered schema
    tracing::info!("Discovered schema: {schema}");
    schema
}
