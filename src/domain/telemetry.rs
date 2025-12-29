// Telemetry data domain models

#[derive(Debug, Clone)]
pub struct TimeSeriesPoint {
    pub time_ms: i64,
    pub value: f64,
}

impl TimeSeriesPoint {
    pub fn new(time_ms: i64, value: f64) -> Self {
        Self { time_ms, value }
    }
}

#[derive(Debug, Clone)]
pub struct TileData {
    pub id: String,
    pub title: String,
    pub unit: String,
    pub value: f64,
    pub precision: i32,
}

impl TileData {
    pub fn new(id: String, title: String, unit: String, value: f64, precision: i32) -> Self {
        Self {
            id,
            title,
            unit,
            value,
            precision,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SeriesData {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub points: Vec<TimeSeriesPoint>,
}

impl SeriesData {
    pub fn new(id: String, name: String, color: Option<String>, points: Vec<TimeSeriesPoint>) -> Self {
        Self {
            id,
            name,
            color,
            points,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChartData {
    pub id: String,
    pub title: String,
    pub unit: Option<String>,
    pub kind: ChartKind,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
    pub fraction_digits: Option<i32>,
    pub series: Vec<SeriesData>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChartKind {
    Line,
    MultiLine,
}

impl ChartData {
    pub fn new(
        id: String,
        title: String,
        unit: Option<String>,
        kind: ChartKind,
        y_min: Option<f64>,
        y_max: Option<f64>,
        fraction_digits: Option<i32>,
        series: Vec<SeriesData>,
    ) -> Self {
        Self {
            id,
            title,
            unit,
            kind,
            y_min,
            y_max,
            fraction_digits,
            series,
        }
    }
}

