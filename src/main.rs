// Main entry point - Dependency injection and server setup
mod domain;
mod application;
mod infrastructure;
mod presentation;

use std::{net::SocketAddr, sync::Arc};
use axum::{routing::get, Router};

use crate::application::aquarium_service::AquariumService;
use crate::application::streaming_service::StreamingDashboardService;
use crate::infrastructure::config::{load_influx_config, load_widgets_config};
use crate::infrastructure::influx_repository::InfluxRepository;
use crate::presentation::app_state::AppState;
use crate::presentation::handlers::{health_check, list_aquariums, stream_dashboard};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let influx_config = load_influx_config()?;
    let widgets_config = load_widgets_config()?;

    // Create repository (infrastructure layer)
    let repository = Arc::new(InfluxRepository::new(
        influx_config.influx.host,
        influx_config.influx.token,
        influx_config.influx.database,
        influx_config.influx.retention_policy,
    ));

    // Create services (application layer)
    let aquarium_service = AquariumService::new(repository.clone());
    let streaming_service = StreamingDashboardService::new(repository.clone(), widgets_config);

    // Create application state
    let state = Arc::new(AppState {
        aquarium_service,
        streaming_service,
    });

    // Build router (presentation layer)
    // Note: We handle compression manually in our response builders,
    // so we don't use CompressionLayer to avoid double compression/decompression
    let router = Router::new()
        .route("/healthz", get(health_check))
        .route("/aquariums", get(list_aquariums))
        .route("/dashboards/:id", get(stream_dashboard))
        .with_state(state);

    // Start server
    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    println!("Starting aquarium-telemetry service on {}", addr);
    
    axum::serve(tokio::net::TcpListener::bind(addr).await?, router).await?;
    
    Ok(())
}

