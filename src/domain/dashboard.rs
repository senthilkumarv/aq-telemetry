// Dashboard domain model
use super::telemetry::{ChartData, TileData};

#[derive(Debug, Clone)]
pub struct Dashboard {
    pub title: String,
    pub tiles: Vec<TileData>,
    pub charts: Vec<ChartData>,
}

impl Dashboard {
    pub fn new(title: String, tiles: Vec<TileData>, charts: Vec<ChartData>) -> Self {
        Self {
            title,
            tiles,
            charts,
        }
    }
}

