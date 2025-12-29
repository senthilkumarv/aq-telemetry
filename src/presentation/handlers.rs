// HTTP request handlers
use crate::infrastructure::chunked_thrift::stream_from_receiver;
use crate::infrastructure::http_response::thrift_list_response;
use crate::presentation::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;
use telemetry_thrift::SDAquarium;

#[derive(Deserialize)]
pub struct RangeQuery {
    pub hours: Option<i32>,
}

/// Health check endpoint
pub async fn health_check() -> &'static str {
    "ok"
}

/// List all aquariums
pub async fn list_aquariums(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Check if client accepts Brotli compression
    let compress = headers
        .get("accept-encoding")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("br"))
        .unwrap_or(false);

    match state.aquarium_service.list_aquariums().await {
        Ok(aquariums) => {
            let thrift_aquariums: Vec<SDAquarium> = aquariums
                .into_iter()
                .map(|a| a.to_thrift())
                .collect();

            match thrift_list_response(thrift_aquariums, compress).await {
                Ok(response) => response,
                Err(status) => status.into_response(),
            }
        }
        Err(e) => {
            eprintln!("Error fetching aquariums: {}", e);
            // Return empty list on error
            match thrift_list_response(Vec::<SDAquarium>::new(), compress).await {
                Ok(response) => response,
                Err(status) => status.into_response(),
            }
        }
    }
}

/// Stream dashboard for a specific aquarium (progressive loading)
pub async fn stream_dashboard(
    Path(id): Path<String>,
    Query(query): Query<RangeQuery>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let hours = query.hours.unwrap_or(6);

    // Check if client accepts Brotli compression
    let compress = headers
        .get("accept-encoding")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("br"))
        .unwrap_or(false);

    let rx = state.streaming_service.stream_dashboard(&id, hours).await;
    stream_from_receiver(rx, compress).await
}

