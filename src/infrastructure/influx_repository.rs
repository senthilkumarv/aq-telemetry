// InfluxDB repository implementation
use crate::application::telemetry_repository::{ProbeMetadata, TelemetryRepository};
use crate::domain::telemetry::TimeSeriesPoint;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct InfluxRepository {
    host: String,
    token: String,
    database: String,
    retention_policy: String,
}

#[derive(Debug, Deserialize)]
struct InfluxQLResponse {
    results: Vec<InfluxQLResult>,
}

#[derive(Debug, Deserialize)]
struct InfluxQLResult {
    #[serde(default)]
    series: Option<Vec<InfluxQLSeries>>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InfluxQLSeries {
    #[allow(dead_code)]
    name: String,
    columns: Vec<String>,
    values: Vec<Vec<serde_json::Value>>,
    #[serde(default)]
    tags: Option<std::collections::HashMap<String, String>>,
}

impl InfluxRepository {
    pub fn new(host: String, token: String, database: String, retention_policy: String) -> Self {
        Self {
            host: host.trim_end_matches('/').to_string(),
            token,
            database,
            retention_policy,
        }
    }

    fn build_query_url(&self, query: &str) -> Result<String> {
        let encoded_query = urlencoding::encode(query);
        Ok(format!(
            "{}/query?db={}&rp={}&q={}",
            self.host, self.database, self.retention_policy, encoded_query
        ))
    }

    async fn execute_query(&self, query: &str) -> Result<InfluxQLResponse> {
        let url = self.build_query_url(query)?;
        
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to send request to InfluxDB")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("InfluxDB query failed with status {}: {}", status, body);
        }

        let data = response
            .json::<InfluxQLResponse>()
            .await
            .context("Failed to parse InfluxDB response")?;

        // Check for errors in the response
        if let Some(result) = data.results.first() {
            if let Some(error) = &result.error {
                anyhow::bail!("InfluxDB query error: {}", error);
            }
        }

        Ok(data)
    }
}

#[async_trait]
impl TelemetryRepository for InfluxRepository {
    async fn list_aquarium_ids(&self) -> Result<Vec<String>> {
        let query = "SHOW TAG VALUES FROM apex_probe WITH KEY = host";
        let response = self.execute_query(query).await?;

        let mut hosts = Vec::new();
        if let Some(result) = response.results.first() {
            if let Some(series) = &result.series {
                for s in series {
                    for value_row in &s.values {
                        if value_row.len() >= 2 {
                            if let Some(host) = value_row[1].as_str() {
                                hosts.push(host.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(hosts)
    }

    async fn get_probe_metadata(&self, aquarium_id: &str, hours: i32) -> Result<Vec<ProbeMetadata>> {
        // Query for actual data points with the host filter, then extract unique combinations
        // This ensures we only get probe_type + name pairs that actually have data for this host
        // Filter based on the selected time range to match what the user is viewing
        // Note: We must SELECT a field (value), not tags. Tags are returned via GROUP BY.
        // Note: "name" is a reserved keyword in InfluxDB, so we must quote it in GROUP BY
        let query = format!(
            "SELECT value FROM apex_probe WHERE host = '{}' AND time >= now() - {}h GROUP BY probe_type, \"name\" LIMIT 1",
            aquarium_id, hours
        );

        tracing::debug!("Executing probe metadata query: {}", query);
        let response = self.execute_query(&query).await?;

        let mut metadata = Vec::new();
        if let Some(result) = response.results.first() {
            if let Some(error) = &result.error {
                tracing::error!("InfluxDB query error: {}", error);
                return Ok(metadata);
            }

            if let Some(series_list) = &result.series {
                tracing::debug!("Got {} series from InfluxDB for host {}", series_list.len(), aquarium_id);

                for series in series_list {
                    // Extract probe_type and name from tags
                    if let Some(tags) = &series.tags {
                        let probe_type = tags.get("probe_type").map(|s| s.to_string());
                        let name = tags.get("name").map(|s| s.to_string());

                        if let (Some(pt), Some(n)) = (probe_type, name) {
                            metadata.push(ProbeMetadata {
                                probe_type: pt,
                                name: n,
                            });
                        }
                    }
                }
            }
        }

        tracing::debug!("Found {} probe metadata entries for host {}", metadata.len(), aquarium_id);
        Ok(metadata)
    }

    async fn query_single_value(&self, query: &str) -> Result<Option<f64>> {
        let response = self.execute_query(query).await?;

        if let Some(result) = response.results.first() {
            if let Some(series) = &result.series {
                if let Some(s) = series.first() {
                    if let Some(value_row) = s.values.first() {
                        // Find the value column (usually index 1 for aggregations)
                        let value_idx = s.columns.iter().position(|c| c == "mean" || c == "last" || c == "value")
                            .unwrap_or(1);
                        
                        if value_idx < value_row.len() {
                            if let Some(val) = value_row[value_idx].as_f64() {
                                return Ok(Some(val));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    async fn query_time_series_downsampled(
        &self,
        query: &str,
        max_points: usize,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let response = self.execute_query(query).await?;

        let mut points = Vec::new();
        if let Some(result) = response.results.first() {
            if let Some(series) = &result.series {
                for s in series {
                    let time_idx = s.columns.iter().position(|c| c == "time").unwrap_or(0);
                    let value_idx = s.columns.iter().position(|c| c == "value").unwrap_or(1);

                    for value_row in &s.values {
                        if value_row.len() > time_idx && value_row.len() > value_idx {
                            if let (Some(time_str), Some(value)) = (
                                value_row[time_idx].as_str(),
                                value_row[value_idx].as_f64(),
                            ) {
                                if let Ok(time) = chrono::DateTime::parse_from_rfc3339(time_str) {
                                    points.push(TimeSeriesPoint::new(
                                        time.timestamp_millis(),
                                        value,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Apply server-side downsampling if needed
        if points.len() > max_points {
            Ok(Self::downsample_points(points, max_points))
        } else {
            Ok(points)
        }
    }
}

impl InfluxRepository {
    /// Downsample time series points using bucket averaging
    fn downsample_points(points: Vec<TimeSeriesPoint>, max_points: usize) -> Vec<TimeSeriesPoint> {
        if points.is_empty() || points.len() <= max_points {
            return points;
        }

        let bucket_size = (points.len() as f64 / max_points as f64).ceil() as usize;
        let mut downsampled = Vec::with_capacity(max_points);

        for chunk_start in (0..points.len()).step_by(bucket_size) {
            let chunk_end = std::cmp::min(chunk_start + bucket_size, points.len());
            let chunk = &points[chunk_start..chunk_end];

            if chunk.is_empty() {
                continue;
            }

            // Use middle point's timestamp and average value
            let mid_idx = chunk.len() / 2;
            let avg_value = chunk.iter().map(|p| p.value).sum::<f64>() / chunk.len() as f64;

            downsampled.push(TimeSeriesPoint::new(chunk[mid_idx].time_ms, avg_value));
        }

        downsampled
    }
}

