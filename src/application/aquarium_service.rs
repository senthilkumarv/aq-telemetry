// Aquarium service - Use case for listing aquariums
use crate::domain::aquarium::Aquarium;
use crate::application::telemetry_repository::TelemetryRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct AquariumService {
    repository: Arc<dyn TelemetryRepository>,
}

impl AquariumService {
    pub fn new(repository: Arc<dyn TelemetryRepository>) -> Self {
        Self { repository }
    }

    pub async fn list_aquariums(&self) -> anyhow::Result<Vec<Aquarium>> {
        let ids = self.repository.list_aquarium_ids().await?;
        Ok(ids.into_iter().map(Aquarium::new).collect())
    }
}

