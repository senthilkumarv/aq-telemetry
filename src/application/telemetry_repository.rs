// Repository trait for telemetry data access
use crate::domain::telemetry::TimeSeriesPoint;
use async_trait::async_trait;

/// Metadata about a probe (probe_type and name)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProbeMetadata {
    pub probe_type: String,
    pub name: String,
}

#[async_trait]
pub trait TelemetryRepository: Send + Sync {
    /// List all available aquarium IDs
    async fn list_aquarium_ids(&self) -> anyhow::Result<Vec<String>>;

    /// Get all probe metadata (probe_type, name) for an aquarium in a single query
    /// Filters based on the selected time range (hours)
    async fn get_probe_metadata(&self, aquarium_id: &str, hours: i32) -> anyhow::Result<Vec<ProbeMetadata>>;

    /// Query a single value (for tiles)
    async fn query_single_value(&self, query: &str) -> anyhow::Result<Option<f64>>;

    /// Query time series data (for charts) with server-side downsampling
    async fn query_time_series_downsampled(
        &self,
        query: &str,
        max_points: usize,
    ) -> anyhow::Result<Vec<TimeSeriesPoint>>;
}

