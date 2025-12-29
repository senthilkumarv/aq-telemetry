// Dashboard service - Use case for building dashboards
use crate::application::telemetry_repository::TelemetryRepository;
use crate::domain::aquarium::Aquarium;
use crate::domain::dashboard::Dashboard;
use crate::domain::telemetry::{ChartData, ChartKind, SeriesData, TileData};
use crate::infrastructure::config::{prepare_query, WidgetsConfig};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct DashboardService {
    repository: Arc<dyn TelemetryRepository>,
    widgets_config: WidgetsConfig,
}

impl DashboardService {
    pub fn new(repository: Arc<dyn TelemetryRepository>, widgets_config: WidgetsConfig) -> Self {
        Self {
            repository,
            widgets_config,
        }
    }

    pub async fn get_dashboard(&self, aquarium_id: &str, hours: i32) -> anyhow::Result<Dashboard> {
        let aquarium = Aquarium::new(aquarium_id.to_string());
        let title = format!("{} Telemetry (last {}h)", aquarium.name, hours);

        // Prepare query variables
        let mut vars = HashMap::new();
        vars.insert("source".to_string(), aquarium_id.to_string());
        vars.insert("hours".to_string(), hours.to_string());

        // Fetch tiles
        let tiles = self.fetch_tiles(&vars).await;

        // Fetch charts
        let charts = self.fetch_charts(&vars).await;

        Ok(Dashboard::new(title, tiles, charts))
    }

    async fn fetch_tiles(&self, vars: &HashMap<String, String>) -> Vec<TileData> {
        let mut tiles = Vec::new();

        for tile_config in &self.widgets_config.tiles {
            let query = prepare_query(&tile_config.query, vars);
            match self.repository.query_single_value(&query).await {
                Ok(Some(value)) => {
                    tiles.push(TileData::new(
                        tile_config.id.clone(),
                        tile_config.title.clone(),
                        tile_config.unit.clone(),
                        value,
                        tile_config.precision,
                    ));
                }
                Ok(None) => {
                    // No data, skip this tile
                }
                Err(e) => {
                    eprintln!("Error fetching tile {}: {}", tile_config.id, e);
                }
            }
        }

        tiles
    }

    async fn fetch_charts(&self, vars: &HashMap<String, String>) -> Vec<ChartData> {
        let mut charts = Vec::new();

        for chart_config in &self.widgets_config.charts {
            let mut series_list = Vec::new();

            for series_config in &chart_config.series {
                let query = prepare_query(&series_config.query, vars);
                match self.repository.query_time_series_downsampled(&query, 150).await {
                    Ok(points) => {
                        if !points.is_empty() {
                            series_list.push(SeriesData::new(
                                series_config.id.clone(),
                                series_config.name.clone(),
                                series_config.color.clone(),
                                points,
                            ));
                        }
                    }
                    Err(e) => {
                        eprintln!("Error fetching series {}: {}", series_config.id, e);
                    }
                }
            }

            // Only add chart if it has at least one series with data
            if !series_list.is_empty() {
                let kind = match chart_config.kind.as_str() {
                    "line" => ChartKind::Line,
                    "multiLine" => ChartKind::MultiLine,
                    _ => ChartKind::Line,
                };

                charts.push(ChartData::new(
                    chart_config.id.clone(),
                    chart_config.title.clone(),
                    chart_config.unit.clone(),
                    kind,
                    chart_config.y_min,
                    chart_config.y_max,
                    chart_config.fraction_digits,
                    series_list,
                ));
            }
        }

        charts
    }
}

