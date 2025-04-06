use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use network_hub::{
    hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus},
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};

#[derive(RustEmbed)]
#[folder = "static"]
struct StaticAssets;

#[derive(Clone)]
struct AppState {
    hub: Arc<Hub>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RouteConfig {
    path: String,
    target: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiConfig {
    path: String,
    response_data: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiRequestData {
    path: String,
    data: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    info!("Starting Network Hub Web App");

    // Print the current working directory for debugging
    let current_dir = std::env::current_dir()?;
    info!("Current working directory: {:?}", current_dir);
    
    // Make sure static directory exists
    let static_dir = PathBuf::from("static");
    if !static_dir.exists() {
        std::fs::create_dir_all(&static_dir)?;
    }
    
    // List all files in the static directory for debugging
    if let Ok(entries) = std::fs::read_dir(&static_dir) {
        info!("Files in static directory:");
        for entry in entries {
            if let Ok(entry) = entry {
                info!("  {:?}", entry.path());
            }
        }
    }

    // Initialize the Hub
    let hub = Hub::initialize(HubScope::Process);
    info!("Network Hub initialized with Process scope");

    // Create the state that will be shared with all routes
    let state = AppState { hub };

    // Set up CORS
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers([header::CONTENT_TYPE]);

    // Create the router
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/assets/*path", get(serve_static_asset))
        .route("/styles.css", get(|| async { serve_static_asset(Path("styles.css".to_string())).await }))
        .route("/main.js", get(|| async { serve_static_asset(Path("main.js".to_string())).await }))
        // API routes for the web interface
        .route("/api/routes", get(get_routes).post(add_route))
        .route("/api/routes/:path", get(get_route).delete(remove_route))
        .route("/api/apis", get(get_apis).post(register_api))
        .route("/api/apis/:path", get(get_api).delete(remove_api))
        .route("/api/request", post(send_api_request))
        .route("/api/hub/stats", get(get_hub_stats))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Web server listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

// Handler for static assets
async fn serve_static_asset(Path(path): Path<String>) -> impl IntoResponse {
    let path_str = path.trim_start_matches('/');
    info!("Requested static asset: {}", path_str);
    
    // First try to serve from the filesystem
    let fs_path = format!("static/{}", path_str);
    match std::fs::read(&fs_path) {
        Ok(content) => {
            info!("Found on filesystem: {}", fs_path);
            let mime = mime_guess::from_path(path_str).first_or_octet_stream();
            let mime_str = mime.as_ref().to_string();
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime_str)],
                content,
            );
        }
        Err(e) => {
            info!("Asset not found on filesystem: {} - {:?}", fs_path, e);
            
            // As fallback, try the embedded assets
            if let Some(content) = StaticAssets::get(path_str) {
                info!("Found embedded asset: {}", path_str);
                let mime = mime_guess::from_path(path_str).first_or_octet_stream();
                let mime_str = mime.as_ref().to_string();
                return (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, mime_str)],
                    content.data.into_owned(),
                );
            }
        }
    }
    
    // If we get here, the asset wasn't found
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/plain".to_string())],
        format!("404 Not Found: {}", path_str).into_bytes(),
    )
}

// Handler for the index page
async fn serve_index() -> impl IntoResponse {
    // For debugging, print all assets in the RustEmbed collection
    info!("Available static assets:");
    for file in StaticAssets::iter() {
        info!("  {}", file);
    }
    
    // Try to read from the filesystem first
    match std::fs::read_to_string("static/index.html") {
        Ok(content) => {
            info!("Serving index.html from filesystem");
            return Html(content);
        },
        Err(e) => {
            info!("Failed to read index.html from filesystem: {:?}", e);
            
            // Try embedded asset as fallback
            if let Some(content) = StaticAssets::get("index.html") {
                info!("Serving index.html from embedded assets");
                if let Ok(html_str) = std::str::from_utf8(&content.data) {
                    return Html(html_str.to_string());
                }
            }
        }
    }
    
    // If we get here, we couldn't find the file
    info!("Could not find index.html in filesystem or embedded assets");
    Html("<h1>Error: Could not load index.html</h1><p>Make sure there is an index.html file in the static directory.</p>".to_string())
}

// API handlers
async fn get_routes(State(_state): State<AppState>) -> impl IntoResponse {
    // This is a placeholder - in a real implementation, we would fetch routes from the proxy
    Json(Vec::<RouteConfig>::new())
}

async fn add_route(
    State(_state): State<AppState>,
    Json(route): Json<RouteConfig>,
) -> impl IntoResponse {
    // Placeholder for adding a route
    info!("Adding route: {} -> {}", route.path, route.target);
    StatusCode::CREATED
}

async fn get_route(
    State(_state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    // Placeholder for getting a specific route
    Json(RouteConfig {
        path: path,
        target: "http://example.com".to_string(),
    })
}

async fn remove_route(
    State(_state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    // Placeholder for removing a route
    info!("Removing route: {}", path);
    StatusCode::NO_CONTENT
}

async fn get_apis(State(_state): State<AppState>) -> impl IntoResponse {
    // Placeholder for getting all APIs
    Json(Vec::<ApiConfig>::new())
}

async fn register_api(
    State(state): State<AppState>,
    Json(api): Json<ApiConfig>,
) -> impl IntoResponse {
    // Register a simple API that always returns the same response
    let response_data = api.response_data.clone();
    let hub = state.hub.clone();
    
    let path = api.path.clone();
    let handler = move |_request: &ApiRequest| {
        ApiResponse {
            data: Box::new(response_data.clone()),
            metadata: HashMap::new(),
            status: ResponseStatus::Success
        }
    };
    
    // Register the API with the hub
    hub.register_api(&path, handler, HashMap::new());
    info!("Registered API: {}", path);
    
    StatusCode::CREATED
}

async fn get_api(
    State(_state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    // Placeholder for getting a specific API
    Json(ApiConfig {
        path,
        response_data: "".to_string(),
    })
}

async fn remove_api(
    State(_state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    // We can't actually remove APIs in the current implementation
    warn!("API removal not implemented: {}", path);
    StatusCode::NOT_IMPLEMENTED
}

async fn send_api_request(
    State(state): State<AppState>,
    Json(request_data): Json<ApiRequestData>,
) -> impl IntoResponse {
    let request = ApiRequest {
        path: request_data.path,
        data: Box::new(request_data.data),
        metadata: HashMap::new(),
        sender_id: "web-client".to_string(),
    };
    
    let response = state.hub.handle_request(request);
    let data = match response.data.downcast::<String>() {
        Ok(string_data) => *string_data,
        Err(_) => "Unable to convert response data to string".to_string(),
    };
    
    Json(serde_json::json!({
        "data": data,
        "status": format!("{:?}", response.status),
    }))
}

async fn get_hub_stats(State(_state): State<AppState>) -> impl IntoResponse {
    // This is a placeholder - in a real implementation, we would fetch statistics from the hub
    Json(serde_json::json!({
        "scope": "Process",
        "api_count": 0,
        "interceptor_count": 0,
    }))
}
