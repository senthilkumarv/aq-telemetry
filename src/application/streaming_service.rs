// Streaming dashboard service - Progressive loading with chunked Thrift
use crate::application::telemetry_repository::{ProbeMetadata, TelemetryRepository};
use crate::infrastructure::config::{prepare_query, WidgetsConfig};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use telemetry_thrift::{
    ChartSkeleton, ChartUpdate, CompletionEvent, DashboardSkeleton, SDPoint, SeriesSkeleton,
    SeriesUpdate, StreamMessage, StreamMessageType, TileSkeleton, TileUpdate,
};
use thrift::OrderedFloat;
use tokio::sync::mpsc;

const MAX_POINTS_PER_SERIES: usize = 150;

#[derive(Clone)]
pub struct StreamingDashboardService {
    repository: Arc<dyn TelemetryRepository>,
    widgets_config: WidgetsConfig,
}

impl StreamingDashboardService {
    pub fn new(repository: Arc<dyn TelemetryRepository>, widgets_config: WidgetsConfig) -> Self {
        Self {
            repository,
            widgets_config,
        }
    }

    pub async fn stream_dashboard(
        &self,
        aquarium_id: &str,
        hours: i32,
    ) -> mpsc::Receiver<StreamMessage> {
        let (tx, rx) = mpsc::channel(100);
        let start_time = Instant::now();

        // 0. Get all probe metadata in a single efficient query
        // Filter based on the selected time range to match what the user is viewing
        let probe_metadata = self
            .repository
            .get_probe_metadata(aquarium_id, hours)
            .await
            .unwrap_or_default();

        // Debug: Log available probes
        tracing::debug!(
            "Available probes for {}: {} probes",
            aquarium_id,
            probe_metadata.len()
        );

        // Build a set for fast lookup
        let available_probes: HashSet<ProbeMetadata> = probe_metadata.into_iter().collect();

        // 1. Build and send skeleton immediately (filtered by available probes)
        let skeleton = self.build_skeleton(aquarium_id, &available_probes);
        let total_widgets = skeleton.tiles.as_ref().map(|t| t.len()).unwrap_or(0)
            + skeleton.charts.as_ref().map(|c| c.len()).unwrap_or(0);

        let skeleton_msg = StreamMessage::new(
            Some(StreamMessageType::SKELETON),
            Some(skeleton),
            None,
            None,
            None,
        );
        let _ = tx.send(skeleton_msg).await;

        // 2. Spawn tasks for tiles (filtered by available probes)
        for tile_config in &self.widgets_config.tiles {
            // Check if this tile's probe exists
            if !self.is_probe_available(&tile_config.query, &available_probes) {
                continue;
            }

            let tx = tx.clone();
            let repo = self.repository.clone();
            let tile_id = tile_config.id.clone();
            let query = self.prepare_tile_query(&tile_config.query, aquarium_id, hours);

            tokio::spawn(async move {
                if let Ok(Some(value)) = repo.query_single_value(&query).await {
                    let update = TileUpdate::new(Some(tile_id), Some(OrderedFloat::from(value)));
                    let msg = StreamMessage::new(
                        Some(StreamMessageType::TILE_UPDATE),
                        None,
                        Some(update),
                        None,
                        None,
                    );
                    let _ = tx.send(msg).await;
                }
            });
        }

        // 3. Spawn tasks for chart series (filtered by available probes, with downsampling)
        for chart_config in &self.widgets_config.charts {
            for series_config in &chart_config.series {
                // Check if this series' probe exists
                if !self.is_probe_available(&series_config.query, &available_probes) {
                    tracing::debug!(
                        "Skipping series {} for chart {} - probe not available",
                        series_config.id, chart_config.id
                    );
                    continue;
                }

                tracing::debug!(
                    "Including series {} for chart {} - probe is available",
                    series_config.id, chart_config.id
                );

                let tx = tx.clone();
                let repo = self.repository.clone();
                let chart_id = chart_config.id.clone();
                let series_id = series_config.id.clone();
                let query = self.prepare_chart_query(&series_config.query, aquarium_id, hours);

                tokio::spawn(async move {
                    // Query with server-side downsampling
                    if let Ok(points) = repo
                        .query_time_series_downsampled(&query, MAX_POINTS_PER_SERIES)
                        .await
                    {
                        // Only send if we have data
                        if !points.is_empty() {
                            let sd_points: Vec<SDPoint> = points
                                .into_iter()
                                .map(|p| {
                                    SDPoint::new(Some(p.time_ms), Some(OrderedFloat::from(p.value)))
                                })
                                .collect();

                            let series_update =
                                SeriesUpdate::new(Some(series_id), Some(sd_points));
                            let chart_update =
                                ChartUpdate::new(Some(chart_id), Some(vec![series_update]));
                            let msg = StreamMessage::new(
                                Some(StreamMessageType::CHART_UPDATE),
                                None,
                                None,
                                Some(chart_update),
                                None,
                            );
                            let _ = tx.send(msg).await;
                        }
                    }
                });
            }
        }

        // 4. Spawn completion task
        let tx_complete = tx.clone();
        tokio::spawn(async move {
            // Give queries time to complete (simple approach - wait a bit)
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            let duration_ms = start_time.elapsed().as_millis() as i64;
            let complete = CompletionEvent::new(Some(total_widgets as i32), Some(duration_ms));
            let msg = StreamMessage::new(
                Some(StreamMessageType::COMPLETE),
                None,
                None,
                None,
                Some(complete),
            );
            let _ = tx_complete.send(msg).await;
        });

        rx
    }

    fn build_skeleton(
        &self,
        aquarium_id: &str,
        available_probes: &HashSet<ProbeMetadata>,
    ) -> DashboardSkeleton {
        // Filter tiles by available probes
        let tiles: Vec<TileSkeleton> = self
            .widgets_config
            .tiles
            .iter()
            .filter(|t| self.is_probe_available(&t.query, available_probes))
            .map(|t| {
                TileSkeleton::new(
                    Some(t.id.clone()),
                    Some(t.title.clone()),
                    Some(t.unit.clone()),
                    Some(t.precision),
                )
            })
            .collect();

        // Filter charts and their series by available probes
        let charts: Vec<ChartSkeleton> = self
            .widgets_config
            .charts
            .iter()
            .filter_map(|c| {
                // Filter series for this chart
                let series: Vec<SeriesSkeleton> = c
                    .series
                    .iter()
                    .filter(|s| self.is_probe_available(&s.query, available_probes))
                    .map(|s| {
                        SeriesSkeleton::new(
                            Some(s.id.clone()),
                            Some(s.name.clone()),
                            s.color.clone(),
                        )
                    })
                    .collect();

                // Only include chart if it has at least one series
                if series.is_empty() {
                    return None;
                }

                let kind = match c.kind.as_str() {
                    "line" => telemetry_thrift::ChartKind::LINE,
                    _ => telemetry_thrift::ChartKind::MULTILINE,
                };

                Some(ChartSkeleton::new(
                    Some(c.id.clone()),
                    Some(c.title.clone()),
                    c.unit.clone(),
                    Some(kind),
                    c.y_min.map(OrderedFloat::from),
                    c.y_max.map(OrderedFloat::from),
                    c.fraction_digits,
                    Some(series),
                ))
            })
            .collect();

        DashboardSkeleton::new(Some(aquarium_id.to_string()), Some(tiles), Some(charts))
    }

    /// Check if a probe exists for this aquarium
    /// - If query has both probe_type and name: checks for exact match
    /// - If query has only probe_type: checks if ANY probe with that type exists
    fn is_probe_available(&self, query: &str, available_probes: &HashSet<ProbeMetadata>) -> bool {
        let probe_type = self.extract_tag_value(query, "probe_type");
        let name = self.extract_tag_value(query, "name");

        match (probe_type, name) {
            // Both probe_type and name specified - check for exact match
            (Some(pt), Some(n)) => {
                let metadata = ProbeMetadata {
                    probe_type: pt.clone(),
                    name: n.clone(),
                };
                let is_available = available_probes.contains(&metadata);

                tracing::debug!(
                    "Checking probe availability: probe_type={}, name={}, available={}",
                    pt, n, is_available
                );

                is_available
            }
            // Only probe_type specified - check if ANY probe with this type exists
            (Some(pt), None) => {
                let is_available = available_probes.iter().any(|p| p.probe_type == pt);

                tracing::debug!(
                    "Checking probe type availability: probe_type={}, available={}",
                    pt, is_available
                );

                is_available
            }
            // No probe_type found - fail open (include the widget)
            _ => {
                tracing::warn!("Could not extract probe_type from query: {}", query);
                true
            }
        }
    }

    /// Extract tag value from InfluxQL query
    /// Example: extract_tag_value(query, "probe_type") from "probe_type"='temp'
    fn extract_tag_value(&self, query: &str, tag_name: &str) -> Option<String> {
        let pattern = format!("\"{}\"='", tag_name);
        if let Some(start) = query.find(&pattern) {
            let start_idx = start + pattern.len();
            if let Some(end_idx) = query[start_idx..].find('\'') {
                return Some(query[start_idx..start_idx + end_idx].to_string());
            }
        }
        None
    }

    fn prepare_tile_query(&self, query: &str, aquarium_id: &str, hours: i32) -> String {
        let mut vars = HashMap::new();
        vars.insert("source".to_string(), aquarium_id.to_string());
        vars.insert("hours".to_string(), hours.to_string());
        prepare_query(query, &vars)
    }

    fn prepare_chart_query(&self, query: &str, aquarium_id: &str, hours: i32) -> String {
        self.prepare_tile_query(query, aquarium_id, hours)
    }
}

