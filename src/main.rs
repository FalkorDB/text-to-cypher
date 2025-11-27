#![allow(clippy::needless_for_each)]

use actix_multipart::Multipart;
use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use actix_web::{App, HttpServer, Responder, Result, post};
use actix_web_lab::sse::{self, Sse};
use falkordb::ConfigValue;
use falkordb::FalkorClientBuilder;
use falkordb::FalkorConnectionInfo;
use futures_util::StreamExt;
use genai::ModelIden;
use genai::resolver::AuthData;
use genai::resolver::AuthResolver;
use moka::sync::Cache;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::mpsc;
use tracing_subscriber::fmt;
use utoipa::OpenApi;
use utoipa::ToSchema;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

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

// Macro for functions returning Option<T>
macro_rules! send_option {
    ($tx:expr, $progress:expr) => {
        match serde_json::to_string(&$progress) {
            Ok(json) => {
                let event = sse::Event::Data(sse::Data::new(json));
                if $tx.send(event).await.is_err() {
                    tracing::warn!("Client disconnected, stopping stream");
                    return None;
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize progress update: {}", e);
                return None;
            }
        }
    };
}

// Macro for functions returning Result<T, ()>
macro_rules! send_result {
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
mod mcp;
mod schema;
mod template;
mod validator;

use chat::{ChatMessage, ChatRequest, ChatRole};
use formatter::{format_as_json, format_query_records};
use mcp::run_mcp_server;
use template::TemplateEngine;
use validator::CypherValidator;

use crate::schema::discovery::Schema;

// Configuration structure for default values from .env file
#[derive(Debug, Clone)]
struct AppConfig {
    falkordb_connection: String,
    default_model: Option<String>,
    default_key: Option<String>,
    schema_cache: Cache<String, String>,
    rest_port: u16,
    mcp_port: u16,
}

static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();

const QUERY_RESULT_MAX_PROPERTY_LENGTH: usize = 100;

impl AppConfig {
    fn load() -> Self {
        // Load .env file if it exists, but don't fail if it doesn't
        let env_loaded = dotenvy::dotenv().is_ok();
        let falkordb_connection =
            std::env::var("FALKORDB_CONNECTION").unwrap_or_else(|_| "falkor://127.0.0.1:6379".to_string());
        let default_model = std::env::var("DEFAULT_MODEL").ok();
        let default_key = std::env::var("DEFAULT_KEY").ok();
        let schema_cache = Cache::new(100);

        let rest_port = std::env::var("REST_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);

        let mcp_port = std::env::var("MCP_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3001);

        tracing::info!(
            "Loaded configuration - env_file_loaded: {}, default_model: {:?}, rest_port: {}, mcp_port: {}",
            env_loaded,
            default_model,
            rest_port,
            mcp_port
        );

        Self {
            falkordb_connection,
            default_model,
            default_key,
            schema_cache,
            rest_port,
            mcp_port,
        }
    }

    fn get() -> &'static Self {
        APP_CONFIG.get_or_init(Self::load)
    }

    /// Check if MCP server should be started based on configuration completeness
    #[allow(clippy::cognitive_complexity)]
    fn should_start_mcp_server(&self) -> bool {
        // Check if both required environment variables are available
        let has_model = self.default_model.is_some();
        let has_key = self.default_key.is_some();

        let should_start = has_model && has_key;

        if should_start {
            tracing::info!("MCP server will be started: both DEFAULT_MODEL and DEFAULT_KEY are configured");
        } else if !has_model {
            tracing::warn!("MCP server not started: DEFAULT_MODEL not set");
        } else if !has_key {
            tracing::warn!("MCP server not started: DEFAULT_KEY not set");
        }

        should_start
    }
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
struct TextToCypherRequest {
    graph_name: String,
    chat_request: ChatRequest,
    model: Option<String>,
    key: Option<String>,
    falkordb_connection: Option<String>,
    /// When true, returns only the generated Cypher query without executing it or generating a final answer
    #[serde(default)]
    #[schema(default = false)]
    cypher_only: bool,
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
            .field("model", &self.model)
            .field("cypher_only", &self.cypher_only);

        if self.key.is_some() {
            debug_struct.field("key", &"***");
        }
        if self.falkordb_connection.is_some() {
            debug_struct.field("falkordb_connection", &"***");
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

#[derive(Serialize, Deserialize, ToSchema)]
struct ConfiguredModelResponse {
    model: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct GraphQueryRequest {
    data: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct GraphListRequest {
    data: Vec<serde_json::Value>,
}

/// Request structure for graph deletion endpoint using Snowflake format
#[derive(Serialize, Deserialize, ToSchema)]
struct GraphDeleteRequest {
    data: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
struct LoadCsvRequest {
    data: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
struct EchoRequest {
    #[serde(flatten)]
    data: serde_json::Value,
}

// Helper function to create Snowflake format error responses
fn create_snowflake_error_response(error_message: &str) -> HttpResponse {
    let error_response = serde_json::json!({
        "data": [
            [0, {"error": error_message}]
        ]
    });
    HttpResponse::BadRequest().json(error_response)
}

fn process_clear_schema_cache(graph_name: &str) {
    tracing::info!("Clearing schema cache for graph: {graph_name}");
    let cache = AppConfig::get().schema_cache.clone();
    cache.invalidate(graph_name);
}

#[utoipa::path(
    get,
    path = "/get_schema/{graph_name}",
    params(
        ("graph_name" = String, Path, description = "Name of the graph to get schema for"),
        ("falkordb_connection" = Option<String>, Query, description = "Optional FalkorDB connection string to override default")
    ),
    responses(
        (status = 200, description = "Graph schema as JSON string", body = String)
    )
)]
#[actix_web::get("/get_schema/{graph_name}")]
async fn get_schema_endpoint(
    graph_name: actix_web::web::Path<String>,
    query: actix_web::web::Query<GetSchemaQuery>,
) -> Result<impl Responder, actix_web::Error> {
    let graph_name = graph_name.into_inner();
    let falkordb_connection = query
        .falkordb_connection
        .as_ref()
        .unwrap_or_else(|| &AppConfig::get().falkordb_connection);

    tracing::info!("Getting schema for graph: {}", graph_name);

    match get_graph_schema_string(falkordb_connection, &graph_name).await {
        Ok(schema) => Ok(HttpResponse::Ok().json(schema)),
        Err(e) => {
            tracing::error!("Failed to get schema for graph {}: {}", graph_name, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to get schema: {}", e)
            })))
        }
    }
}

#[utoipa::path(
    get,
    path = "/configured-model",
    responses(
        (status = 200, description = "Configured default model", body = ConfiguredModelResponse),
        (status = 200, description = "DEFAULT_MODEL is not set", body = ErrorResponse)
    )
)]
#[actix_web::get("/configured-model")]
async fn configured_model_endpoint() -> Result<impl Responder, actix_web::Error> {
    let config = AppConfig::get();

    config.default_model.as_ref().map_or_else(
        || {
            Ok(HttpResponse::Ok().json(ErrorResponse {
                error: "DEFAULT_MODEL is not set".to_string(),
            }))
        },
        |model| Ok(HttpResponse::Ok().json(ConfiguredModelResponse { model: model.clone() })),
    )
}

#[allow(clippy::cognitive_complexity)]
#[utoipa::path(
    post,
    path = "/graph_query",
    request_body = GraphQueryRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = String, content_type = "application/json"),
        (status = 400, description = "Query execution failed", body = ErrorResponse)
    )
)]
#[post("/graph_query")]
async fn graph_query_endpoint(
    req: actix_web::web::Json<GraphQueryRequest>
) -> Result<impl Responder, actix_web::Error> {
    let raw_request = req.into_inner();

    // Log the incoming Snowflake format request
    tracing::info!("Received graph_query request with Snowflake format");
    tracing::info!(
        "Raw JSON payload: {}",
        serde_json::to_string_pretty(&raw_request).unwrap_or_else(|_| "Failed to serialize".to_string())
    );

    // Validate the Snowflake format: data should be an array with at least one entry
    if raw_request.data.is_empty() {
        tracing::error!("Empty data array in Snowflake request");
        return Ok(create_snowflake_error_response("Data array cannot be empty"));
    }

    // Get the first entry from the data array
    let first_entry = &raw_request.data[0];

    // Snowflake format: data[0] should be an array where [0] is index and [1] is the actual data
    let data_array = first_entry.as_array().ok_or_else(|| {
        tracing::error!("First data entry is not an array");
        actix_web::error::ErrorBadRequest("First data entry must be an array")
    })?;

    if data_array.len() < 2 {
        tracing::error!("Data array must have at least 2 elements [index, data]");
        return Ok(create_snowflake_error_response(
            "Data array must have at least 2 elements [index, data]",
        ));
    }

    // Extract the actual data object (second element in the array)
    let data_object = &data_array[1];

    tracing::info!(
        "Extracted data object: {}",
        serde_json::to_string_pretty(data_object).unwrap_or_else(|_| "Failed to serialize".to_string())
    );

    // Extract the required fields from the data object
    let graph_name = data_object
        .get("graph_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            tracing::error!("Missing or invalid 'graph_name' field in data object");
            actix_web::error::ErrorBadRequest("Missing or invalid 'graph_name' field")
        })?
        .to_string();

    let query = data_object
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            tracing::error!("Missing or invalid 'query' field in data object");
            actix_web::error::ErrorBadRequest("Missing or invalid 'query' field")
        })?
        .to_string();

    tracing::info!("Successfully extracted: graph_name={}, query={}", graph_name, query);

    // Validate the extracted data
    if graph_name.is_empty() {
        tracing::warn!("Empty graph name provided");
        return Ok(create_snowflake_error_response("Graph name cannot be empty"));
    }

    if query.is_empty() {
        tracing::warn!("Empty query provided");
        return Ok(create_snowflake_error_response("Query cannot be empty"));
    }

    // Execute the query
    match graph_query(&query, &graph_name, false).await {
        Ok(json_result) => {
            tracing::info!("Successfully executed graph_query for graph: {}", graph_name);
            tracing::debug!("Raw query result: {}", json_result);

            // Parse the JSON result to convert it to Snowflake format
            match serde_json::from_str::<serde_json::Value>(&json_result) {
                Ok(parsed_result) => {
                    // Convert the result to Snowflake format: { "data": [ [0, result] ] }
                    let snowflake_response = serde_json::json!({
                        "data": [
                            [0, parsed_result]
                        ]
                    });

                    tracing::info!(
                        "Converted to Snowflake format: {}",
                        serde_json::to_string_pretty(&snowflake_response)
                            .unwrap_or_else(|_| "Failed to serialize".to_string())
                    );

                    Ok(HttpResponse::Ok().json(snowflake_response))
                }
                Err(e) => {
                    tracing::error!("Failed to parse query result as JSON: {}", e);
                    // If parsing fails, return the raw result wrapped in Snowflake format
                    let snowflake_response = serde_json::json!({
                        "data": [
                            [0, json_result]
                        ]
                    });
                    Ok(HttpResponse::Ok().json(snowflake_response))
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to execute graph_query for graph {}: {}", graph_name, e);
            Ok(create_snowflake_error_response(&e.to_string()))
        }
    }
}

#[utoipa::path(
    post,
    path = "/graph_list",
    request_body = GraphListRequest,
    responses(
        (status = 200, description = "List of available graphs", body = String, content_type = "application/json"),
        (status = 400, description = "Failed to list graphs", body = ErrorResponse)
    )
)]
#[post("/graph_list")]
#[allow(clippy::cognitive_complexity)]
async fn graph_list_endpoint(_req: actix_web::web::Json<GraphListRequest>) -> Result<impl Responder, actix_web::Error> {
    // Get the list of graphs
    match get_graphs_list().await {
        Ok(graphs) => {
            tracing::info!("Successfully retrieved {} graphs", graphs.len());
            tracing::debug!("Graph list: {:?}", graphs);

            // Convert the graph list to Snowflake format: { "data": [ [0, graph_names_array] ] }
            let snowflake_response = serde_json::json!({
                "data": [
                    [0, graphs]
                ]
            });

            tracing::info!(
                "Converted to Snowflake format: {}",
                serde_json::to_string_pretty(&snowflake_response).unwrap_or_else(|_| "Failed to serialize".to_string())
            );

            Ok(HttpResponse::Ok().json(snowflake_response))
        }
        Err(e) => {
            tracing::error!("Failed to list graphs: {}", e);
            Ok(create_snowflake_error_response(&format!("Failed to list graphs: {e}")))
        }
    }
}

#[utoipa::path(
    post,
    path = "/graph_delete",
    request_body = GraphDeleteRequest,
    responses(
        (status = 200, description = "Graph deleted successfully", body = String, content_type = "application/json"),
        (status = 400, description = "Failed to delete graph", body = ErrorResponse)
    )
)]
#[post("/graph_delete")]
#[allow(clippy::cognitive_complexity)]
async fn graph_delete_endpoint(
    req: actix_web::web::Json<GraphDeleteRequest>
) -> Result<impl Responder, actix_web::Error> {
    let raw_request = req.into_inner();

    // Log the incoming Snowflake format request
    tracing::info!("Received graph_delete request with Snowflake format");
    tracing::info!(
        "Raw JSON payload: {}",
        serde_json::to_string_pretty(&raw_request).unwrap_or_else(|_| "Failed to serialize".to_string())
    );

    // Validate the Snowflake format: data should be an array with at least one entry
    if raw_request.data.is_empty() {
        tracing::error!("Empty data array in Snowflake request");
        return Ok(create_snowflake_error_response("Data array cannot be empty"));
    }

    // Get the first entry from the data array
    let first_entry = &raw_request.data[0];

    // Snowflake format: data[0] should be an array where [0] is index and [1] is the actual data
    let data_array = first_entry.as_array().ok_or_else(|| {
        tracing::error!("First data entry is not an array");
        actix_web::error::ErrorBadRequest("First data entry must be an array")
    })?;

    if data_array.len() < 2 {
        tracing::error!("Data array must have at least 2 elements [index, data]");
        return Ok(create_snowflake_error_response(
            "Data array must have at least 2 elements [index, data]",
        ));
    }

    // Extract the actual data object (second element in the array)
    let data_object = &data_array[1];

    tracing::info!(
        "Extracted data object: {}",
        serde_json::to_string_pretty(data_object).unwrap_or_else(|_| "Failed to serialize".to_string())
    );

    // Extract the required graph_name field from the data object
    let graph_name = data_object
        .get("graph_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            tracing::error!("Missing or invalid 'graph_name' field in data object");
            actix_web::error::ErrorBadRequest("Missing or invalid 'graph_name' field")
        })?
        .to_string();

    tracing::info!("Successfully extracted graph_name: {}", graph_name);

    // Validate the extracted data
    if graph_name.is_empty() {
        tracing::warn!("Empty graph name provided");
        return Ok(create_snowflake_error_response("Graph name cannot be empty"));
    }

    // Delete the graph
    match delete_graph(&graph_name).await {
        Ok(result) => {
            tracing::info!("Successfully deleted graph: {}", graph_name);
            tracing::debug!("Delete result: {}", result);

            // Convert the result to Snowflake format: { "data": [ [0, result] ] }
            let snowflake_response = serde_json::json!({
                "data": [
                    [0, {"message": format!("Graph '{}' deleted successfully", graph_name), "success": true}]
                ]
            });

            tracing::info!(
                "Converted to Snowflake format: {}",
                serde_json::to_string_pretty(&snowflake_response).unwrap_or_else(|_| "Failed to serialize".to_string())
            );

            Ok(HttpResponse::Ok().json(snowflake_response))
        }
        Err(e) => {
            tracing::error!("Failed to delete graph {}: {}", graph_name, e);
            Ok(create_snowflake_error_response(&format!(
                "Failed to delete graph '{graph_name}': {e}"
            )))
        }
    }
}

#[utoipa::path(
    post,
    path = "/graph_query_upload/{graph_name}",
    params(
        ("graph_name" = String, Path, description = "Name of the graph to execute query on")
    ),
    request_body(content = String, description = "Multipart form data with 'file' and 'cypher' fields", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Query executed successfully with uploaded CSV", body = String, content_type = "application/json"),
        (status = 400, description = "Query execution failed or invalid form data", body = ErrorResponse)
    )
)]
#[post("/graph_query_upload/{graph_name}")]
#[allow(clippy::future_not_send)]
async fn graph_query_upload_endpoint(
    graph_name: actix_web::web::Path<String>,
    mut payload: Multipart,
) -> Result<impl Responder, actix_web::Error> {
    let graph_name = graph_name.into_inner();

    let mut csv_content: Option<String> = None;
    let mut cypher_query: Option<String> = None;

    // Process multipart data field by field
    while let Some(item) = futures_util::stream::StreamExt::next(&mut payload).await {
        let mut field =
            item.map_err(|e| actix_web::error::ErrorBadRequest(format!("Failed to read multipart field: {e}")))?;

        // Get the field name
        let field_name = field.content_disposition().get_name().map(ToString::to_string);

        if let Some(field_name) = field_name {
            // Read the field data into bytes
            let mut bytes = actix_web::web::BytesMut::new();
            while let Some(chunk) = futures_util::stream::StreamExt::next(&mut field).await {
                let data =
                    chunk.map_err(|e| actix_web::error::ErrorBadRequest(format!("Failed to read field chunk: {e}")))?;
                bytes.extend_from_slice(&data);
            }

            // Convert to string
            let content = String::from_utf8(bytes.to_vec()).map_err(|e| {
                actix_web::error::ErrorBadRequest(format!("Invalid UTF-8 in field '{field_name}': {e}"))
            })?;

            // Store the content based on field name
            match field_name.as_str() {
                "file" => csv_content = Some(content),
                "cypher" => cypher_query = Some(content),
                _ => tracing::warn!("Unexpected field in multipart data: {}", field_name),
            }
        }
    }

    // Validate that we have both required fields
    let csv_content =
        csv_content.ok_or_else(|| actix_web::error::ErrorBadRequest("Missing 'file' field in multipart data"))?;
    let cypher_query =
        cypher_query.ok_or_else(|| actix_web::error::ErrorBadRequest("Missing 'cypher' field in multipart data"))?;

    // Execute the query with uploaded CSV data
    match graph_query_with_csv(&cypher_query, &graph_name, &csv_content).await {
        Ok(json_result) => Ok(HttpResponse::Ok().content_type("application/json").body(json_result)),
        Err(e) => Ok(HttpResponse::BadRequest().json(ErrorResponse { error: e.to_string() })),
    }
}

#[utoipa::path(
    get,
    path = "/list_graphs",
    responses(
        (status = 200, description = "List of available graphs", body = Vec<String>)
    )
)]
#[actix_web::get("/list_graphs")]
async fn list_graphs_endpoint() -> Result<impl Responder, actix_web::Error> {
    match get_graphs_list().await {
        Ok(graphs) => Ok(HttpResponse::Ok().json(graphs)),
        Err(e) => {
            tracing::error!("Failed to list graphs: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to list graphs: {}", e)
            })))
        }
    }
}

#[utoipa::path(
    post,
    path = "/clear_schema_cache/{graph_name}",
    params(
        ("graph_name" = String, Path, description = "Name of the graph to clear from cache")
    ),
    responses(
        (status = 200, description = "Schema cache cleared successfully")
    )
)]
#[post("/clear_schema_cache/{graph_name}")]
async fn clear_schema_cache(graph_name: actix_web::web::Path<String>) -> impl Responder {
    let graph_name = graph_name.into_inner();
    tracing::info!("Clearing schema cache for graph: {}", graph_name);
    process_clear_schema_cache(&graph_name);
    HttpResponse::new(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/load_csv",
    request_body = LoadCsvRequest,
    responses(
        (status = 200, description = "CSV file loaded and query executed successfully", body = String, content_type = "application/json"),
        (status = 400, description = "Invalid request format, CSV file not found, or query execution failed", body = ErrorResponse)
    )
)]
#[allow(clippy::too_many_lines)]
#[allow(clippy::cognitive_complexity)]
#[post("/load_csv")]
async fn load_csv_endpoint(req: actix_web::web::Json<LoadCsvRequest>) -> Result<impl Responder, actix_web::Error> {
    let raw_request = req.into_inner();

    // Log the incoming Snowflake format request
    tracing::info!("Received load_csv request with Snowflake format");
    tracing::info!(
        "Raw JSON payload: {}",
        serde_json::to_string_pretty(&raw_request).unwrap_or_else(|_| "Failed to serialize".to_string())
    );

    // List all files in IMPORT_FOLDER at the start
    if let Ok(connection_info) = AppConfig::get().falkordb_connection.as_str().try_into() {
        if let Ok(client) = FalkorClientBuilder::new_async()
            .with_connection_info(connection_info)
            .build()
            .await
        {
            match list_import_folder_files(&client).await {
                Ok(files) => {
                    tracing::info!("Files currently in IMPORT_FOLDER: {:?}", files);
                    if files.is_empty() {
                        tracing::info!("IMPORT_FOLDER is empty");
                    } else {
                        tracing::info!("Total files in IMPORT_FOLDER: {}", files.len());
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to list IMPORT_FOLDER files: {}", e);
                }
            }
        } else {
            tracing::warn!("Failed to create FalkorDB client for listing IMPORT_FOLDER files");
        }
    } else {
        tracing::warn!("Invalid FalkorDB connection string for listing IMPORT_FOLDER files");
    }

    // Validate the Snowflake format: data should be an array with at least one entry
    if raw_request.data.is_empty() {
        tracing::error!("Empty data array in Snowflake request");
        return Ok(create_snowflake_error_response("Data array cannot be empty"));
    }

    // Get the first entry from the data array
    let first_entry = &raw_request.data[0];

    // Snowflake format: data[0] should be an array where [0] is index and [1] is the actual data
    let data_array = first_entry.as_array().ok_or_else(|| {
        tracing::error!("First data entry is not an array");
        actix_web::error::ErrorBadRequest("First data entry must be an array")
    })?;

    if data_array.len() < 2 {
        tracing::error!("Data array must have at least 2 elements [index, data]");
        return Ok(create_snowflake_error_response(
            "Data array must have at least 2 elements [index, data]",
        ));
    }

    // Extract the actual data object (second element in the array)
    let data_object = &data_array[1];

    tracing::info!(
        "Extracted data object: {}",
        serde_json::to_string_pretty(data_object).unwrap_or_else(|_| "Failed to serialize".to_string())
    );

    // Extract the required fields from the data object
    let csv_file = data_object
        .get("csv_file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            tracing::error!("Missing or invalid 'csv_file' field in data object");
            actix_web::error::ErrorBadRequest("Missing or invalid 'csv_file' field")
        })?
        .to_string();

    let cypher_query = data_object
        .get("cypher_query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            tracing::error!("Missing or invalid 'cypher_query' field in data object");
            actix_web::error::ErrorBadRequest("Missing or invalid 'cypher_query' field")
        })?
        .to_string();

    let graph_name = data_object
        .get("graph_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            tracing::error!("Missing or invalid 'graph_name' field in data object");
            actix_web::error::ErrorBadRequest("Missing or invalid 'graph_name' field")
        })?
        .to_string();

    tracing::info!(
        "Successfully extracted: graph_name={}, csv_file={}, cypher_query={}",
        graph_name,
        csv_file,
        cypher_query
    );
    tracing::debug!("CSV file: {}", csv_file);

    tracing::info!(
        "Successfully extracted: graph_name={}, csv_file={}, cypher_query={}",
        graph_name,
        csv_file,
        cypher_query
    );
    tracing::debug!("CSV file: {}", csv_file);

    // Validate the extracted data
    if csv_file.is_empty() {
        tracing::warn!("Empty CSV file name provided");
        return Ok(create_snowflake_error_response("CSV file name cannot be empty"));
    }

    if cypher_query.is_empty() {
        tracing::warn!("Empty Cypher query provided");
        return Ok(create_snowflake_error_response("Cypher query cannot be empty"));
    }

    if graph_name.is_empty() {
        tracing::warn!("Empty graph name provided");
        return Ok(create_snowflake_error_response("Graph name cannot be empty"));
    }

    // Execute the query with the existing CSV file using the new logic
    match graph_query_with_existing_csv(&cypher_query, &graph_name, &csv_file).await {
        Ok(json_result) => {
            tracing::info!("Successfully executed load_csv for graph: {}", graph_name);
            tracing::debug!("Raw query result: {}", json_result);

            // Parse the JSON result to convert it to Snowflake format
            match serde_json::from_str::<serde_json::Value>(&json_result) {
                Ok(parsed_result) => {
                    // Convert the result to Snowflake format: { "data": [ [0, result] ] }
                    let snowflake_response = serde_json::json!({
                        "data": [
                            [0, parsed_result]
                        ]
                    });

                    tracing::info!(
                        "Converted to Snowflake format: {}",
                        serde_json::to_string_pretty(&snowflake_response)
                            .unwrap_or_else(|_| "Failed to serialize".to_string())
                    );

                    Ok(HttpResponse::Ok().json(snowflake_response))
                }
                Err(e) => {
                    tracing::error!("Failed to parse query result as JSON: {}", e);
                    // If parsing fails, return the raw result wrapped in Snowflake format
                    let snowflake_response = serde_json::json!({
                        "data": [
                            [0, json_result]
                        ]
                    });
                    Ok(HttpResponse::Ok().json(snowflake_response))
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to execute load_csv for graph {}: {}", graph_name, e);
            Ok(create_snowflake_error_response(&e.to_string()))
        }
    }
}

#[utoipa::path(
    post,
    path = "/echo",
    request_body = EchoRequest,
    responses(
        (status = 200, description = "Echo back the received JSON", content_type = "application/json")
    )
)]
#[post("/echo")]
async fn echo_endpoint(req: actix_web::web::Json<serde_json::Value>) -> Result<impl Responder, actix_web::Error> {
    // Log the incoming request
    tracing::info!("Echo endpoint called");
    tracing::info!(
        "Received JSON payload: {}",
        serde_json::to_string_pretty(&*req).unwrap_or_else(|_| "Failed to serialize".to_string())
    );

    // Simply return the received JSON back
    let response = HttpResponse::Ok().json(&*req);

    tracing::info!("Echo endpoint responding with same payload");
    Ok(response)
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
    let config = AppConfig::get();

    // Apply defaults from .env file if values are not provided
    if request.model.is_none() {
        request.model.clone_from(&config.default_model);
    }

    if request.key.is_none() {
        request.key.clone_from(&config.default_key);
    }

    let (tx, rx) = mpsc::channel(100);

    // Ensure we have a model after applying defaults
    if request.model.is_none() {
        // Send error via SSE instead of returning HTTP error
        tokio::spawn(async move {
            let error_event = sse::Event::Data(sse::Data::new(
                serde_json::to_string(&Progress::Error(
                    "Model must be provided either in request or as DEFAULT_MODEL in .env file".to_string(),
                ))
                .unwrap_or_else(|_| r#"{"Error":"Serialization failed"}"#.to_string()),
            ));
            let _ = tx.send(error_event).await;
        });
        let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(Ok::<_, actix_web::Error>);
        return Ok(Sse::from_stream(stream));
    }

    let model = request.model.as_ref().unwrap(); // Safe to unwrap after the check above

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

    // Handle service target resolution errors via SSE
    let service_target = match client.resolve_service_target(model).await {
        Ok(target) => target,
        Err(e) => {
            // Send error via SSE instead of returning HTTP error
            tokio::spawn(async move {
                let error_event = sse::Event::Data(sse::Data::new(
                    serde_json::to_string(&Progress::Error(format!("Failed to resolve service target: {e}")))
                        .unwrap_or_else(|_| r#"{"Error":"Serialization failed"}"#.to_string()),
                ));
                let _ = tx.send(error_event).await;
            });
            let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(Ok::<_, actix_web::Error>);
            return Ok(Sse::from_stream(stream));
        }
    };

    tokio::spawn(async move {
        process_text_to_cypher_request(request, client, service_target, tx).await;
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(Ok::<_, actix_web::Error>);

    Ok(Sse::from_stream(stream))
}

#[allow(clippy::cognitive_complexity)]
async fn process_text_to_cypher_request(
    request: TextToCypherRequest,
    client: genai::Client,
    service_target: genai::ServiceTarget,
    tx: mpsc::Sender<sse::Event>,
) {
    tracing::info!("Processing text to Cypher request: {request:?}");

    let model = request
        .model
        .as_ref()
        .expect("Model should be available after applying defaults");

    let falkordb_connection = request
        .clone()
        .falkordb_connection
        .unwrap_or_else(|| AppConfig::get().falkordb_connection.clone());

    // Step 1: Send processing status
    send_processing_status(&request, &service_target, &tx).await;

    // Step 2: Discover schema
    let Some(schema) = get_or_discover_schema(&falkordb_connection, &request.graph_name, &tx).await else {
        send!(tx, Progress::Error("Failed to discover schema".to_string()));
        return;
    };

    // Step 3: Generate and execute cypher query with self-healing retry
    let Some(initial_query) = generate_cypher_query(&request, &schema, &client, model, &tx).await else {
        return;
    };
    let mut executed_query = initial_query.clone();

    // If cypher_only is true, stop here and return just the validated query
    if request.cypher_only {
        tracing::info!("Query preview mode: returning generated query without execution");
        send!(tx, Progress::Result(executed_query));
        return;
    }

    // Step 4: Execute the query and get results, with self-healing on failure
    let query_result = if let Ok(result) =
        execute_cypher_query(&executed_query, &request.graph_name, falkordb_connection.as_str(), &tx).await
    {
        tracing::info!("first before query_result: {}", result);
        result  
    } else {
        // Try self-healing: regenerate query with error feedback
        tracing::info!("First query execution failed, attempting self-healing...");
        send!(
            tx,
            Progress::Status(String::from("Query failed, attempting self-healing..."))
        );

        // Use a generic error message since we don't capture specific errors
        let error_msg = "Query execution failed - see logs for details";

        // Attempt to get a fixed query with error context
        if let Some(fixed_query) =
            attempt_query_self_healing(&request, &schema, &executed_query, error_msg, &client, model, &tx).await
        {
            // Try executing the fixed query
            if let Ok(result) =
                execute_cypher_query(&fixed_query, &request.graph_name, falkordb_connection.as_str(), &tx).await
            {
                tracing::info!("Self-healed query executed successfully");
                send!(tx, Progress::Status(String::from("Self-healing successful")));
                executed_query = fixed_query;
                result
            } else {
                tracing::error!("Self-healing failed");
                send!(
                    tx,
                    Progress::Error("Query execution failed even after self-healing attempt".to_string())
                );
                return;
            }
        } else {
            return;
        }
    };

    // Step 5: Generate final answer using AI
    generate_final_answer(&request, &executed_query, &query_result, &client, model, &tx).await;
}

/// Validates a query and returns it if valid, None otherwise
#[allow(clippy::cognitive_complexity)]
async fn validate_and_log_query(
    query: &str,
    tx: &mpsc::Sender<sse::Event>,
) -> Option<String> {
    let validation_result = CypherValidator::validate(query);

    if !validation_result.is_valid {
        tracing::warn!("Query failed validation: {:?}", validation_result.errors);
        send_option!(
            tx,
            Progress::Error(format!(
                "Query validation errors: {}",
                validation_result.errors.join("; ")
            ))
        );
        return None;
    }

    // Log any warnings even if query is valid
    if !validation_result.warnings.is_empty() {
        tracing::info!("Query validation warnings: {:?}", validation_result.warnings);
    }

    Some(query.to_string())
}

/// Attempts to self-heal a failed query by regenerating with error context
#[allow(clippy::cognitive_complexity)]
async fn attempt_query_self_healing(
    request: &TextToCypherRequest,
    schema: &str,
    failed_query: &str,
    error_message: &str,
    client: &genai::Client,
    model: &str,
    tx: &mpsc::Sender<sse::Event>,
) -> Option<String> {
    tracing::info!("Attempting to self-heal failed query: {}", failed_query);

    // Create a feedback message with specific error context
    let mut retry_request = request.chat_request.clone();
    retry_request.messages.push(ChatMessage {
        role: ChatRole::Assistant,
        content: failed_query.to_string(),
    });
    retry_request.messages.push(ChatMessage {
        role: ChatRole::User,
        content: format!(
            "The previous query failed with error: {error_message}. Please generate a corrected Cypher query that fixes this error and follows the schema more closely."
        ),
    });

    // Generate new query
    let genai_chat_request = generate_create_cypher_query_chat_request(&retry_request, schema);
    let retry_query = execute_chat(client, model, genai_chat_request, tx).await;

    if retry_query.trim().is_empty() || retry_query.trim() == "NO ANSWER" {
        tracing::warn!("Self-healing failed: no valid query generated");
        return None;
    }

    let clean_query = retry_query.replace('\n', " ").replace("```", "").trim().to_string();

    // Validate the regenerated query using shared validation logic
    if let Some(validated) = validate_and_log_query(&clean_query, tx).await {
        send_option!(tx, Progress::CypherQuery(format!("Fixed: {validated}")));
        Some(validated)
    } else {
        None
    }
}

async fn get_or_discover_schema(
    falkordb_connection: &str,
    graph_name: &str,
    tx: &mpsc::Sender<sse::Event>,
) -> Option<String> {
    let cache = AppConfig::get().schema_cache.clone();
    let schema = match cache.get(graph_name) {
        Some(schema) => schema,
        None => match discover_and_send_schema(falkordb_connection, graph_name, tx).await {
            Ok(schema) => schema,
            Err(()) => return None,
        },
    };
    send_option!(tx, Progress::Schema(schema.clone()));
    cache.insert(graph_name.to_string(), schema.clone());
    Some(schema.clone())
}

#[allow(clippy::cognitive_complexity)]
async fn generate_cypher_query(
    request: &TextToCypherRequest,
    schema: &str,
    client: &genai::Client,
    model: &str,
    tx: &mpsc::Sender<sse::Event>,
) -> Option<String> {
    send_option!(
        tx,
        Progress::Status(String::from("Generating Cypher query using schema ..."))
    );

    let genai_chat_request = generate_create_cypher_query_chat_request(&request.chat_request, schema);
    let query = execute_chat(client, model, genai_chat_request, tx).await;

    if query.trim().is_empty() || query.trim() == "NO ANSWER" {
        tracing::warn!("No query generated from AI model");
        send_option!(tx, Progress::Error("No valid query was generated".to_string()));
        return None;
    }

    let clean_query = query.replace('\n', " ").replace("```", "").trim().to_string();

    // Validate the generated query using shared validation logic
    if validate_and_log_query(&clean_query, tx).await.is_none() {
        send_option!(
            tx,
            Progress::Status(String::from("Query validation failed, attempting to regenerate..."))
        );

        // Try to regenerate with error feedback
        let validation_result = CypherValidator::validate(&clean_query);
        let error_feedback = validation_result.errors.join("; ");
        let retry_request = append_validation_feedback(&request.chat_request, &clean_query, &error_feedback);
        let genai_chat_request = generate_create_cypher_query_chat_request(&retry_request, schema);
        let retry_query = execute_chat(client, model, genai_chat_request, tx).await;

        if !retry_query.trim().is_empty() && retry_query.trim() != "NO ANSWER" {
            let retry_clean = retry_query.replace('\n', " ").replace("```", "").trim().to_string();

            // Use shared validation for retry as well
            if let Some(validated) = validate_and_log_query(&retry_clean, tx).await {
                tracing::info!("Retry query passed validation");
                send_option!(tx, Progress::CypherQuery(validated.clone()));
                return Some(validated);
            }
        }

        // If retry failed, still use original but warn
        send_option!(
            tx,
            Progress::Status(String::from("Warning: Query validation issues detected"))
        );
    }

    send_option!(tx, Progress::CypherQuery(clean_query.clone()));
    Some(clean_query)
}

#[allow(clippy::cognitive_complexity)]
async fn execute_cypher_query(
    query: &str,
    graph_name: &str,
    falkordb_connection: &str,
    tx: &mpsc::Sender<sse::Event>,
) -> Result<String, ()> {
    send_result!(tx, Progress::Status(String::from("Executing Cypher query...")));
    tracing::info!("Executing Cypher Query: {}", query);

    match execute_query(query, graph_name, falkordb_connection, true, tx).await {
        Ok(result) => {
            tracing::info!("Query executed successfully, result: {}", result);
            send_result!(tx, Progress::CypherResult(result.clone()));
            Ok(result)
        }
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!("Query execution failed: {}", error_msg);
            send_result!(tx, Progress::Error(format!("Query execution failed: {error_msg}")));
            Err(())
        }
    }
}

async fn generate_final_answer(
    request: &TextToCypherRequest,
    query: &str,
    query_result: &str,
    client: &genai::Client,
    model: &str,
    tx: &mpsc::Sender<sse::Event>,
) {
    let sanitized_result = sanitize_query_result(query_result, QUERY_RESULT_MAX_PROPERTY_LENGTH);
    if sanitized_result != query_result {
        tracing::debug!("Query result sanitized before sending to AI model");
    }
    tracing::info!("query_result: {}", sanitized_result);
    send!(
        tx,
        Progress::Status(String::from(
            "Generating answer from chat history and Cypher output using AI model..."
        ))
    );
    let genai_chat_request = generate_answer_chat_request(&request.chat_request, query, &sanitized_result);
    execute_chat_stream(client, model, genai_chat_request, tx).await;
}

fn sanitize_query_result(query_result: &str, max_len: usize) -> String {
    let truncate = |text: &str| -> String {
        if text.chars().count() <= max_len {
            text.to_string()
        } else {
            let truncated: String = text.chars().take(max_len).collect();
            format!("{truncated}...")
        }
    };

    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(query_result) {
        let mut stack = vec![&mut value];
        while let Some(current) = stack.pop() {
            match current {
                serde_json::Value::String(s) => {
                    if s.chars().count() > max_len {
                        let truncated: String = s.chars().take(max_len).collect();
                        *s = format!("{truncated}...");
                    }
                }
                serde_json::Value::Array(arr) => stack.extend(arr.iter_mut()),
                serde_json::Value::Object(map) => stack.extend(map.values_mut()),
                _ => {}
            }
        }
        return serde_json::to_string(&value).unwrap_or_else(|_| truncate(query_result));
    }

    if query_result.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(query_result.len());
    let mut idx = 0;
    let mut in_string = false;
    let mut escape = false;
    let bytes = query_result.as_bytes();

    while idx < query_result.len() {
        let ch = query_result[idx..].chars().next().unwrap();

        if ch == '\\' && !escape {
            escape = true;
        } else {
            if ch == '"' && !escape {
                in_string = !in_string;
            }
            escape = false;
        }

        if ch == '[' && !in_string {
            let window_start = idx.saturating_sub(256);
            let window = &query_result[window_start..idx];
            if let Some(colon_pos) = window.rfind(':') {
                let suffix = &window[colon_pos..];
                if !suffix.contains('\n') && !suffix.contains('\r') {
                    let mut depth = 0usize;
                    let mut end = idx;
                    let mut matched = false;

                    while end < bytes.len() {
                        match bytes[end] {
                            b'[' => depth += 1,
                            b']' => {
                                if depth == 0 {
                                    break;
                                }
                                depth -= 1;
                                if depth == 0 {
                                    matched = true;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        end += 1;
                    }

                    if matched {
                        let inner = &query_result[idx + 1..end];
                        if inner.chars().count() > max_len {
                            let truncated: String = inner.chars().take(max_len).collect();
                            result.push('[');
                            result.push_str(&truncated);
                            result.push_str("...");
                            result.push(']');
                        } else {
                            result.push_str(&query_result[idx..=end]);
                        }
                        idx = end + 1;
                        continue;
                    }
                }
            }
        }

        result.push(ch);
        idx += ch.len_utf8();
    }

    if result.is_empty() {
        truncate(query_result)
    } else {
        result
    }
}

#[allow(dead_code)]
async fn graph_query(
    query: &str,
    graph_name: &str,
    read_only: bool,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let connection_info: FalkorConnectionInfo = AppConfig::get()
        .falkordb_connection
        .as_str()
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
    let result = tokio::task::spawn_blocking(move || execute_query_blocking(&client, &graph_name, &query, read_only))
        .await
        .map_err(|e| format!("Failed to execute blocking task: {e}"))?;

    let json_result = match result {
        Ok(records) => format_as_json(&records),
        Err(e) => {
            let error_msg = format!("Query execution failed: {e}");
            return Err(error_msg.into());
        }
    };
    Ok(json_result)
}

async fn graph_query_with_csv(
    query: &str,
    graph_name: &str,
    csv_content: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(
        "graph_query_with_csv called with graph_name: {}, query: {}, csv_content length: {}",
        graph_name,
        query,
        csv_content.len()
    );

    let connection_info: FalkorConnectionInfo = AppConfig::get()
        .falkordb_connection
        .as_str()
        .try_into()
        .map_err(|e| format!("Invalid connection info: {e}"))?;

    let client = FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let graph_name = graph_name.to_string();
    let query = query.to_string();
    let csv_content = csv_content.to_string();

    // replace filename in the query with a random uuid.
    let uuid = Uuid::new_v4().to_string();
    let filename = format!("{uuid}.csv");
    let re = Regex::new(r"file://.*\.csv").unwrap();
    let query = re.replace(&query, format!("file://{uuid}.csv")).to_string();

    tracing::info!("Extracted CSV filename from query: {filename}");
    tracing::info!("query is: {query}");

    // Run the FalkorDB operations in a blocking context
    let result = tokio::task::spawn_blocking(move || {
        execute_query_with_csv_import_blocking(&client, &graph_name, &query, &csv_content, &filename)
    })
    .await
    .map_err(|e| format!("Failed to execute blocking task: {e}"))?;

    let json_result = match result {
        Ok(records) => format_as_json(&records),
        Err(e) => {
            let error_msg = format!("Query execution failed: {e}");
            return Err(error_msg.into());
        }
    };
    Ok(json_result)
}

async fn graph_query_with_existing_csv(
    query: &str,
    graph_name: &str,
    csv_filename: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(
        "graph_query_with_existing_csv called with graph_name: {}, query: {}, csv_filename: {}",
        graph_name,
        query,
        csv_filename
    );

    let connection_info: FalkorConnectionInfo = AppConfig::get()
        .falkordb_connection
        .as_str()
        .try_into()
        .map_err(|e| format!("Invalid connection info: {e}"))?;

    let client = FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let graph_name = graph_name.to_string();
    let csv_filename = csv_filename.to_string();

    // Replace filename patterns in the query with the actual CSV filename
    let re = Regex::new(r"file://.*\.csv").unwrap();
    let updated_query = re.replace_all(query, format!("file://{csv_filename}")).to_string();

    tracing::info!("Original query: {}", query);
    tracing::info!("Updated query with actual filename: {}", updated_query);

    // Run the FalkorDB operations in a blocking context
    let result = tokio::task::spawn_blocking(move || {
        execute_query_with_existing_csv_blocking(&client, &graph_name, &updated_query, &csv_filename)
    })
    .await
    .map_err(|e| format!("Failed to execute blocking task: {e}"))?;

    let json_result = match result {
        Ok(records) => format_as_json(&records),
        Err(e) => {
            let error_msg = format!("Query execution failed: {e}");
            return Err(error_msg.into());
        }
    };
    Ok(json_result)
}

async fn execute_query(
    query: &str,
    graph_name: &str,
    falkordb_connection: &str,
    read_only: bool,
    tx: &mpsc::Sender<sse::Event>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let connection_info: FalkorConnectionInfo = falkordb_connection
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
    let result = tokio::task::spawn_blocking(move || execute_query_blocking(&client, &graph_name, &query, read_only))
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

async fn get_graph_schema_string(
    falkordb_connection: &str,
    graph_name: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let cache = AppConfig::get().schema_cache.clone();

    // Check cache first
    if let Some(cached_schema) = cache.get(graph_name) {
        return Ok(cached_schema);
    }

    // If not in cache, discover it
    let schema = discover_graph_schema(falkordb_connection, graph_name).await;
    let schema_json = serde_json::to_string(&schema).map_err(|e| format!("Failed to serialize schema: {e}"))?;

    // Cache the result
    cache.insert(graph_name.to_string(), schema_json.clone());

    Ok(schema_json)
}

async fn get_graphs_list() -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let connection_info: FalkorConnectionInfo = AppConfig::get()
        .falkordb_connection
        .as_str()
        .try_into()
        .map_err(|e| format!("Invalid connection info: {e}"))?;

    let client = FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
        .map_err(|e| format!("Failed to build client: {e}"))?;

    // Call the async version directly
    let graphs = client.list_graphs().await.map_err(|e| format!("Failed to list graphs: {e}"))?;
    Ok(graphs)
}

/// Deletes a graph from `FalkorDB`
///
/// # Arguments
///
/// * `graph_name` - The name of the graph to delete
///
/// # Returns
///
/// * `Result<String, Box<dyn std::error::Error + Send + Sync>>` - Success message or error
///
/// # Errors
///
/// This function will return an error if:
/// - The connection to `FalkorDB` fails
/// - The graph deletion operation fails
/// - The graph does not exist
async fn delete_graph(graph_name: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let connection_info: FalkorConnectionInfo = AppConfig::get()
        .falkordb_connection
        .as_str()
        .try_into()
        .map_err(|e| format!("Invalid connection info: {e}"))?;

    let client = FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let graph_name_owned = graph_name.to_string();

    // Run the FalkorDB operations in a blocking context
    tokio::task::spawn_blocking(move || {
        // Create a new Tokio runtime for this blocking operation
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;

        rt.block_on(async {
            // Select the graph and call delete on it
            let mut graph = client.select_graph(&graph_name_owned);
            graph.delete().await.map_err(|e| format!("Failed to delete graph: {e}"))?;

            Ok::<String, Box<dyn std::error::Error + Send + Sync>>(format!(
                "Graph '{graph_name_owned}' deleted successfully"
            ))
        })
    })
    .await
    .map_err(|e| format!("Failed to execute blocking task: {e}"))?
}

fn execute_query_blocking(
    client: &falkordb::FalkorAsyncClient,
    graph_name: &str,
    query: &str,
    read_only: bool,
) -> Result<Vec<Vec<falkordb::FalkorValue>>, Box<dyn std::error::Error + Send + Sync>> {
    // Create a new Tokio runtime for this blocking operation
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;

    rt.block_on(async {
        let mut graph = client.select_graph(graph_name);
        let query_result = if read_only {
            graph
                .ro_query(query)
                .execute()
                .await
                .map_err(|e| format!("Query execution failed: {e}"))?
        } else {
            graph
                .query(query)
                .execute()
                .await
                .map_err(|e| format!("Query execution failed: {e}"))?
        };

        let mut records = Vec::new();
        for record in query_result.data {
            records.push(record);
        }
        Ok(records)
    })
}

fn execute_query_with_csv_import_blocking(
    client: &falkordb::FalkorAsyncClient,
    graph_name: &str,
    query: &str,
    csv_content: &str,
    filename: &str,
) -> Result<Vec<Vec<falkordb::FalkorValue>>, Box<dyn std::error::Error + Send + Sync>> {
    use std::fs;
    use std::path::PathBuf;

    // Create a new Tokio runtime for this blocking operation
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;

    rt.block_on(async {
        // Get the IMPORT_FOLDER using graph.config get IMPORT_FOLDER
        let import_folder = get_import_folder(client).await?;
        tracing::info!("FalkorDB IMPORT_FOLDER config: {}", import_folder);

        // Check current user and directory permissions
        let current_user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        tracing::info!("Running as user: {}", current_user);

        // Check if import folder exists and its permissions
        let path = PathBuf::from(&import_folder);
        if path.exists() {
            tracing::info!("IMPORT_FOLDER already exists: {}", import_folder);
            if let Ok(metadata) = fs::metadata(&import_folder) {
                tracing::info!("IMPORT_FOLDER permissions: {:?}", metadata.permissions());
            }
        } else {
            tracing::info!("IMPORT_FOLDER does not exist, attempting to create: {}", import_folder);
            fs::create_dir_all(&import_folder).map_err(|e| {
                tracing::error!("Failed to create IMPORT_FOLDER '{}': {}", import_folder, e);
                format!("Failed to create IMPORT_FOLDER: {e}")
            })?;
            tracing::info!("Successfully created IMPORT_FOLDER: {}", import_folder);
        }

        tracing::info!("Using IMPORT_FOLDER: {}", import_folder);
        // Create the full file path
        let file_path = PathBuf::from(&import_folder).join(filename);

        tracing::info!("Full file path for CSV import: {:?}", file_path);

        // Write CSV content to the import folder
        fs::write(&file_path, csv_content).map_err(|e| format!("Failed to write CSV file to import folder: {e}"))?;
        tracing::info!("CSV file written to import folder: {:?}", file_path);

        // Execute the query (no need to modify the query as the file is now in the correct location)
        let mut graph = client.select_graph(graph_name);
        let query_result = graph
            .query(query)
            .execute()
            .await
            .map_err(|e| format!("Query execution failed: {e}"))?;

        tracing::info!("Query {query} executed, processing results...");

        let mut records = Vec::new();
        for record in query_result.data {
            records.push(record);
        }

        tracing::info!(
            "Query executed successfully with CSV import, records count: {}",
            records.len()
        );
        tracing::info!("Cleaning up CSV file: {:?}", file_path);
        // Clean up - delete the file from the IMPORT_FOLDER
        if let Err(e) = fs::remove_file(&file_path) {
            tracing::warn!("Failed to remove CSV file from import folder: {}", e);
        }

        Ok(records)
    })
}

fn execute_query_with_existing_csv_blocking(
    client: &falkordb::FalkorAsyncClient,
    graph_name: &str,
    query: &str,
    csv_filename: &str,
) -> Result<Vec<Vec<falkordb::FalkorValue>>, Box<dyn std::error::Error + Send + Sync>> {
    use std::path::PathBuf;

    // Create a new Tokio runtime for this blocking operation
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;

    rt.block_on(async {
        // Get the IMPORT_FOLDER using graph.config get IMPORT_FOLDER
        let import_folder = get_import_folder(client).await?;
        tracing::info!("FalkorDB IMPORT_FOLDER config: {}", import_folder);

        // Create the full file path
        let file_path = PathBuf::from(&import_folder).join(csv_filename);
        tracing::info!("Expected CSV file path: {:?}", file_path);

        // Check if the file exists
        if !file_path.exists() {
            let error_msg = format!("CSV file '{csv_filename}' not found in IMPORT_FOLDER '{import_folder}'");
            tracing::error!("{}", error_msg);
            return Err(error_msg.into());
        }

        tracing::info!("CSV file found at: {:?}", file_path);

        // Read and log each line of the CSV file
        match std::fs::read_to_string(&file_path) {
            Ok(csv_content) => {
                let lines: Vec<&str> = csv_content.lines().collect();
                tracing::info!("CSV file '{}' contains {} lines", csv_filename, lines.len());

                for (line_number, line) in lines.iter().enumerate() {
                    let line_num = line_number + 1; // 1-based line numbering
                    tracing::info!("CSV line {}: {}", line_num, line);
                }

                if lines.is_empty() {
                    tracing::warn!("CSV file '{}' is empty", csv_filename);
                } else {
                    tracing::info!(
                        "Finished logging all {} lines from CSV file '{}'",
                        lines.len(),
                        csv_filename
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read CSV file '{}' for line logging: {}", csv_filename, e);
                // Continue with execution even if reading for logging fails
            }
        }

        // Execute the query (the file is already in the correct location)
        let mut graph = client.select_graph(graph_name);
        let query_result = graph
            .query(query)
            .execute()
            .await
            .map_err(|e| format!("Query execution failed: {e}"))?;

        tracing::info!("Query {query} executed, processing results...");

        let mut records = Vec::new();
        for record in query_result.data {
            records.push(record);
        }

        tracing::info!(
            "Query executed successfully with existing CSV file, records count: {}",
            records.len()
        );

        Ok(records)
    })
}

#[allow(clippy::cognitive_complexity)]
async fn get_import_folder(
    client: &falkordb::FalkorAsyncClient
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // First check if IMPORT_FOLDER environment variable is defined
    if let Ok(env_import_folder) = std::env::var("IMPORT_FOLDER") {
        tracing::info!("IMPORT_FOLDER found in environment variable");
        tracing::info!("Using IMPORT_FOLDER from environment: {}", env_import_folder);
        return Ok(env_import_folder);
    }

    // Fall back to existing logic - query FalkorDB configuration
    tracing::info!("IMPORT_FOLDER not found in environment, attempting to get from FalkorDB configuration");
    let values = client.config_get("IMPORT_FOLDER").await.map_err(|e| {
        tracing::error!("Failed to get IMPORT_FOLDER config from FalkorDB: {}", e);
        format!("Failed to get IMPORT_FOLDER from FalkorDB: {e}")
    })?;

    tracing::info!("Received FalkorDB config values: {:?}", values);

    let config_value: ConfigValue = values
        .get("IMPORT_FOLDER")
        .cloned()
        .ok_or("IMPORT_FOLDER not found in FalkorDB config response")?;

    match config_value {
        ConfigValue::String(s) => {
            tracing::info!(
                "Successfully retrieved IMPORT_FOLDER from FalkorDB configuration: {}",
                s
            );
            Ok(s)
        }
        ConfigValue::Int64(_) => {
            tracing::error!("IMPORT_FOLDER from FalkorDB is not a string");
            Err("IMPORT_FOLDER from FalkorDB is not a string".into())
        }
    }
}

async fn list_import_folder_files(
    client: &falkordb::FalkorAsyncClient
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    use std::fs;

    tracing::info!("Attempting to list files in IMPORT_FOLDER");
    let import_folder = get_import_folder(client).await?;
    tracing::info!("IMPORT_FOLDER path: {}", import_folder);

    let entries = fs::read_dir(&import_folder).map_err(|e| {
        tracing::error!("Failed to read IMPORT_FOLDER directory '{}': {}", import_folder, e);
        format!("Failed to read IMPORT_FOLDER directory: {e}")
    })?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();

        if path.is_file()
            && let Some(file_name) = path.file_name().and_then(|name| name.to_str())
        {
            files.push(file_name.to_string());
        }
    }

    files.sort(); // Sort alphabetically for consistent output
    tracing::info!("Found {} files in IMPORT_FOLDER: {:?}", files.len(), files);

    Ok(files)
}

/// Appends validation feedback to a chat request for query regeneration
fn append_validation_feedback(
    chat_request: &ChatRequest,
    failed_query: &str,
    error_message: &str,
) -> ChatRequest {
    let mut messages = chat_request.messages.clone();

    // Add the failed query as an assistant message
    messages.push(ChatMessage {
        role: ChatRole::Assistant,
        content: failed_query.to_string(),
    });

    // Add validation error as user feedback
    messages.push(ChatMessage {
        role: ChatRole::User,
        content: format!(
            "The previous query has validation errors: {error_message}. Please generate a corrected Cypher query."
        ),
    });

    ChatRequest { messages }
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

#[allow(clippy::pedantic)]
#[derive(OpenApi)]
#[openapi(
    paths(
        text_to_cypher,
        clear_schema_cache,
        load_csv_endpoint,
        echo_endpoint,
        list_graphs_endpoint,
        graph_list_endpoint,
        graph_delete_endpoint,
        get_schema_endpoint,
        configured_model_endpoint,
        graph_query_endpoint,
        graph_query_upload_endpoint
    ),
    components(schemas(
        TextToCypherRequest,
        Progress,
        ChatRequest,
        ChatMessage,
        ChatRole,
        ConfiguredModelResponse,
        ErrorResponse,
        GraphQueryRequest,
        GraphListRequest,
        GraphDeleteRequest,
        LoadCsvRequest,
        EchoRequest,
        error::ErrorResponse
    ))
)]
struct ApiDoc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    fmt().with_max_level(tracing::Level::INFO).init();

    // Initialize configuration from .env file
    let config = AppConfig::get();
    let rest_port = config.rest_port;
    let mcp_port = config.mcp_port;

    tracing::info!(
        "Starting server with REST API on port {} and MCP on port {}",
        rest_port,
        mcp_port
    );

    // Conditionally start MCP server based on configuration
    let mcp_handle = if config.should_start_mcp_server() {
        Some(tokio::spawn(async move {
            if let Err(e) = run_mcp_server(mcp_port).await {
                tracing::error!("MCP server error: {}", e);
            }
        }))
    } else {
        None
    };

    // Start the HTTP server with Swagger UI at /swagger-ui/
    // OpenAPI documentation will be available at /api-doc/openapi.json
    // Swagger UI will be accessible at:
    // http://localhost:{rest_port}/swagger-ui/

    tracing::info!("Starting HTTP server on 0.0.0.0:{}", rest_port);

    let http_server = HttpServer::new(|| {
        App::new()
            .service(text_to_cypher)
            .service(clear_schema_cache)
            .service(load_csv_endpoint)
            .service(echo_endpoint)
            .service(list_graphs_endpoint)
            .service(graph_list_endpoint)
            .service(graph_delete_endpoint)
            .service(get_schema_endpoint)
            .service(configured_model_endpoint)
            .service(graph_query_endpoint)
            .service(graph_query_upload_endpoint)
            .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-doc/openapi.json", ApiDoc::openapi()))
    })
    .bind(("0.0.0.0", rest_port))?
    .run();

    // Run server(s) concurrently
    if let Some(mcp_handle) = mcp_handle {
        // Run both HTTP and MCP servers
        tokio::select! {
            result = http_server => {
                tracing::info!("HTTP server stopped");
                result
            }
            _ = mcp_handle => {
                tracing::info!("MCP server stopped");
                Ok(())
            }
        }
    } else {
        // Run only HTTP server
        tracing::info!("Running HTTP server only");
        let result = http_server.await;
        tracing::info!("HTTP server stopped");
        result
    }
}

#[derive(Deserialize)]
struct GetSchemaQuery {
    falkordb_connection: Option<String>,
}

async fn discover_graph_schema(
    falkordb_connection: &str,
    graph_name: &str,
) -> Schema {
    let connection_info: FalkorConnectionInfo = falkordb_connection.try_into().expect("Invalid connection info");

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
    falkordb_connection: &str,
    graph_name: &str,
    tx: &mpsc::Sender<sse::Event>,
) -> Result<String, ()> {
    try_send!(
        tx,
        Progress::Status(format!("Discovering schema for graph: {graph_name}"))
    );

    let schema = discover_graph_schema(falkordb_connection, graph_name).await;

    // Serialize and handle errors inline
    let Ok(json_schema) = serde_json::to_string(&schema) else {
        tracing::error!("Failed to serialize schema to JSON");
        try_send!(tx, Progress::Error("Failed to serialize schema".to_string()));
        return Err(());
    };

    tracing::info!("Discovered schema: {}", json_schema);
    Ok(json_schema)
}

async fn send_processing_status(
    request: &TextToCypherRequest,
    service_target: &genai::ServiceTarget,
    tx: &mpsc::Sender<sse::Event>,
) {
    let adapter_kind = service_target.model.adapter_kind;
    let model_name = request.model.as_deref().unwrap_or("unknown");
    send!(
        tx,
        Progress::Status(format!(
            "Processing query for graph: {} using model: {} ({:?})",
            request.graph_name, model_name, adapter_kind
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

    let content = chat_response
        .content_text_into_string()
        .unwrap_or_else(|| String::from("NO ANSWER"));

    tracing::info!("Generated chat response: {}", content);
    content
}

async fn execute_chat_stream(
    client: &genai::Client,
    model: &str,
    genai_chat_request: genai::chat::ChatRequest,
    tx: &mpsc::Sender<sse::Event>,
) -> String {
    if let Ok(pretty_json) = serde_json::to_string_pretty(&genai_chat_request) {
        tracing::info!("Streaming genai chat request:\n{}", pretty_json);
    } else {
        tracing::info!("Streaming genai chat request: {:?}", genai_chat_request);
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that `cypher_only` defaults to `false` when not specified in the JSON
    #[test]
    fn test_cypher_only_defaults_to_false() {
        let json = r#"{
            "graph_name": "test_graph",
            "chat_request": {
                "messages": [{"role": "user", "content": "Test question"}]
            }
        }"#;

        let request: TextToCypherRequest = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(
            !request.cypher_only,
            "cypher_only should default to false when not specified"
        );
    }

    /// Test that `cypher_only` is correctly deserialized when explicitly set to `true`
    #[test]
    fn test_cypher_only_true_when_specified() {
        let json = r#"{
            "graph_name": "test_graph",
            "chat_request": {
                "messages": [{"role": "user", "content": "Test question"}]
            },
            "cypher_only": true
        }"#;

        let request: TextToCypherRequest = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(request.cypher_only, "cypher_only should be true when explicitly set");
    }

    /// Test that `cypher_only` is correctly deserialized when explicitly set to `false`
    #[test]
    fn test_cypher_only_false_when_specified() {
        let json = r#"{
            "graph_name": "test_graph",
            "chat_request": {
                "messages": [{"role": "user", "content": "Test question"}]
            },
            "cypher_only": false
        }"#;

        let request: TextToCypherRequest = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(
            !request.cypher_only,
            "cypher_only should be false when explicitly set to false"
        );
    }

    /// Test that the request serializes correctly with `cypher_only` set to `true`
    #[test]
    fn test_cypher_only_serialization() {
        let request = TextToCypherRequest {
            graph_name: "test_graph".to_string(),
            chat_request: ChatRequest {
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "Test question".to_string(),
                }],
            },
            model: None,
            key: None,
            falkordb_connection: None,
            cypher_only: true,
        };

        let json = serde_json::to_string(&request).expect("Failed to serialize");
        assert!(
            json.contains("\"cypher_only\":true"),
            "Serialized JSON should contain cypher_only: true"
        );
    }

    /// Test that optional fields work correctly with `cypher_only`
    #[test]
    fn test_cypher_only_with_optional_fields() {
        let json = r#"{
            "graph_name": "test_graph",
            "chat_request": {
                "messages": [{"role": "user", "content": "Test question"}]
            },
            "model": "gpt-4",
            "key": "test-api-key",
            "falkordb_connection": "falkor://localhost:6379",
            "cypher_only": true
        }"#;

        let request: TextToCypherRequest = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(request.cypher_only, "cypher_only should be true");
        assert_eq!(request.model, Some("gpt-4".to_string()));
        assert_eq!(request.key, Some("test-api-key".to_string()));
        assert_eq!(request.falkordb_connection, Some("falkor://localhost:6379".to_string()));
    }
}
