// Application state for HTTP handlers
use crate::application::aquarium_service::AquariumService;
use crate::application::streaming_service::StreamingDashboardService;

#[derive(Clone)]
pub struct AppState {
    pub aquarium_service: AquariumService,
    pub streaming_service: StreamingDashboardService,
}



